//! Module to format captured lock events as html.

use std::collections::{BTreeMap, HashMap};
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

use crate::event::{EventBacktrace, EventId, EventKind};
use crate::Event;

type Child<'a> = (EventId, &'a str, u64, Option<&'a EventBacktrace>);

const STYLE: &[u8] = include_bytes!("trace.css");
const SCRIPT: &[u8] = include_bytes!("trace.js");

/// Write events to the given path.
pub fn write<P>(path: P, events: &[Event]) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    let file_stem = path.file_stem().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing file stem from the specified path",
        )
    })?;

    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing parent from the specified path",
        )
    })?;

    let css = parent.join(file_stem).with_extension("css");
    let script = parent.join(file_stem).with_extension("js");

    std::fs::write(&css, STYLE)?;
    std::fs::write(&script, SCRIPT)?;

    let css = css
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid css file name"))?;

    let script = script
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid script file name"))?;

    let mut out = std::fs::File::create(path)?;

    // Start of trace.
    let mut start = u64::MAX;
    // End of trace.
    let mut end = u64::MIN;

    let mut opens = BTreeMap::<_, BTreeMap<_, Vec<_>>>::new();
    let mut children = HashMap::<_, Vec<_>>::new();
    let mut closes = HashMap::new();

    for event in events {
        start = start.min(event.timestamp);
        end = end.max(event.timestamp);

        match &event.kind {
            EventKind::Enter {
                lock,
                name,
                type_name,
                parent,
                backtrace,
                ..
            } => {
                if let Some(parent) = parent {
                    children.entry(*parent).or_default().push((
                        event.id,
                        name.as_ref(),
                        event.timestamp,
                        backtrace.as_ref(),
                    ));
                } else {
                    opens
                        .entry((*lock, type_name.as_ref()))
                        .or_default()
                        .entry(event.thread_index)
                        .or_default()
                        .push((event.id, name.as_ref(), event.timestamp, backtrace.as_ref()));
                }
            }
            EventKind::Leave {
                sibling: Some(sibling),
            } => {
                closes.insert(*sibling, event.timestamp);
            }
            _ => {}
        }
    }

    if start == u64::MAX {
        return Ok(());
    }

    writeln!(out, "<!DOCTYPE html>")?;
    writeln!(out, "<html>")?;
    writeln!(out, "<head>")?;
    writeln!(out, r#"<link href="{css}" rel="stylesheet">"#)?;
    writeln!(out, "</head>")?;

    writeln!(out, "<body>")?;
    writeln!(out, "<div id=\"traces\">")?;

    for ((lock, type_name), events) in opens {
        writeln!(out, "<div class=\"lock-instance\">")?;

        let kind = lock.kind();
        let index = lock.index();

        let type_name = type_name.replace('<', "&lt;").replace('>', "&gt");

        writeln!(
            out,
            r#"<div class="title">{kind:?}&lt;{type_name}&gt; (lock index: {index})</div>"#
        )?;

        writeln!(out, "<div class=\"lock-session\">")?;

        for (thread_index, events) in events.into_iter() {
            let start = events
                .iter()
                .map(|(_, _, start, _)| *start)
                .min()
                .unwrap_or(0);

            let end = events
                .iter()
                .flat_map(|(id, _, _, _)| closes.get(id).copied())
                .max()
                .unwrap_or(0);

            writeln!(
                out,
                r#"<div data-toggle="event-{lock}-{thread_index}-details" data-start="{start}" data-end="{end}" class="timeline">"#
            )?;

            let mut details = Vec::new();

            for (id, name, open, backtrace) in events {
                let Some(close) = closes.get(&id).copied() else {
                    return Ok(());
                };

                writeln! {
                    details,
                    r#"
                    <tr data-entry data-entry-start="{open}" data-entry-close="{close}">
                        <td class="title" colspan="6">Event: {id}</td>
                    </tr>
                    "#
                }?;

                write_section(
                    &mut out,
                    id,
                    (start, end),
                    name,
                    open,
                    close,
                    &children,
                    &closes,
                    backtrace,
                    &mut details,
                )?;
            }

            writeln!(
                out,
                r#"<span class="section-heading"><span>{thread_index}</span></span>"#
            )?;

            writeln!(out, r#"<div class="timeline-target"></div>"#)?;
            writeln!(out, "</div>")?;

            if !details.is_empty() {
                writeln!(
                    out,
                    r#"<table id="event-{lock}-{thread_index}-details" class="details">"#
                )?;

                out.write_all(&details)?;
                writeln!(out, "</table>")?;
            }
        }

        writeln!(out, "</div>")?;
        writeln!(out, "</div>")?;
    }

    writeln!(out, "</div>")?;
    writeln!(
        out,
        r#"<script type="text/javascript" src="{script}"></script>"#
    )?;
    writeln!(out, "</body>")?;
    writeln!(out, "</html>")?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_section(
    out: &mut dyn io::Write,
    id: EventId,
    span: (u64, u64),
    title: &str,
    open: u64,
    close: u64,
    children: &HashMap<EventId, Vec<Child<'_>>>,
    closes: &HashMap<EventId, u64>,
    backtrace: Option<&EventBacktrace>,
    d: &mut Vec<u8>,
) -> io::Result<()> {
    let (start, end) = span;

    if start == end {
        return Ok(());
    }

    let total = (end - start) as f32;

    let left = (((open - start) as f32 / total) * 100.0).round() as u32;
    let width = (((close - open) as f32 / total) * 100.0).round() as u32;

    let s = Duration::from_nanos(open);
    let e = Duration::from_nanos(close);
    let duration = Duration::from_nanos(close - open);

    let style = format!("width: {width}%; left: {left}%;");
    let hover_title = format!("{title} ({s:?}-{e:?})");

    writeln!(
        out,
        "<div id=\"event-{id}\" class=\"section {title}\" style=\"{style}\" title=\"{hover_title}\"></div>"
    )?;

    writeln! {
        d,
        r#"
        <tr data-entry data-entry-start="{open}" data-entry-close="{close}">
            <td class="title {title}">{title}</td>
            <td>{s:?}</td>
            <td>&mdash;</td>
            <td>{e:?}</td>
            <td>({duration:?})</td>
            <td width="100%"></td>
        </tr>
        "#
    }?;

    if let Some(backtrace) = backtrace {
        writeln!(
            d,
            r#"<tr><td>Backtrace:</td><td class="backtrace" colspan="5">{backtrace}</td></tr>"#
        )?;
    }

    for &(id, title, child_open, backtrace) in children.get(&id).into_iter().flatten() {
        let Some(child_close) = closes.get(&id).copied() else {
            continue;
        };

        write_section(
            out,
            id,
            span,
            title,
            child_open,
            child_close,
            children,
            closes,
            backtrace,
            d,
        )?;
    }

    Ok(())
}
