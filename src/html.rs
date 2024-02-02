//! Module to format captured lock events as html.

use std::collections::{BTreeMap, HashMap};
use std::io::{self, Write};
use std::time::Duration;

use crate::event::{EventBacktrace, EventId, EventKind};
use crate::Event;

type Child<'a> = (EventId, &'a str, u64, Option<&'a EventBacktrace>);

const STYLE: &str = r#"
html {
    font-family: helvetica, arial, sans-serif;
}

#traces {
    width: 100%;
}

.title {
    font-size: 14px;
    font-weight: bold;
}

.lock-instance {
    border: 1px solid #808080;
    padding: 10px;
    background-color: #f0f0f0;
    margin: 10px 0;
}

.lock-instance:last-child {
    margin-top: 0;
}

.timeline {
    font-size: 18px;
    position: relative;
    min-height: 1em;
    background-color: #ffffff;
    margin-top: 10px;
}

.lock-session {
    cursor: pointer;
    margin: 5px 0;
}

.lock-session:last-child {
    margin-bottom: 0;
}

.section {
    position: absolute;
    min-width: 2px;
    height: 1em;
}

.section-heading {
    position: absolute;
    left: 0;
}

.section-heading span {
    font-size: 0.6em;
    line-height: 1.66em;
    padding-left: 0.166em;
}

.section.critical {
    background-color: #e0e0e0;
}

.section.read {
    background-color: #367336;
}

.section.write {
    background-color: #ff8080;
}

.section.lock {
    background-color: #ff80ff;
}

.title.critical {
    color: #808080;
}

.title.read {
    color: #367336;
}

.title.write {
    color: #ff8080;
}

.title.lock {
    color: #ff80ff;
}

.details {
    font-size: 12px;
    padding: 5px;
    margin-top: 10px;
}

.details td {
    padding: 2px;
}

.backtrace {
    font-family: monospace;
    font-size: 12px;
    white-space: pre;
}
"#;

const SCRIPT: &str = r#"
let visibleTarget = null;

function toggle(element) {
    if (element.style.display === "none") {
        if (!!visibleTarget) {
            visibleTarget.style.display = "none";
            visibleTarget = null;
        }  

        element.style.display = "block";
        visibleTarget = element;
    } else {
        element.style.display = "none";
        visibleTarget = null;
    }
}

document.querySelectorAll('[data-toggle]').forEach(function(element) {
    let id = element.getAttribute("data-toggle");
    let target = document.getElementById(id);

    if (!!target) {
        element.addEventListener("click", function(e) {
            toggle(target);
        });
    }
});
"#;

/// Write a sequence of events to html.
pub fn write<O>(mut out: O, events: &[Event]) -> io::Result<()>
where
    O: io::Write,
{
    // Start of trace.
    let mut start = u64::MAX;
    // End of trace.
    let mut end = u64::MIN;

    let mut opens = BTreeMap::<_, Vec<_>>::new();
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
                    opens.entry((*lock, type_name.as_ref())).or_default().push((
                        event.id,
                        name.as_ref(),
                        event.timestamp,
                        backtrace.as_ref(),
                        event.thread_index,
                    ));
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

    writeln!(out, "<html>")?;
    writeln!(out, "<head>")?;
    writeln!(out, "<style>{STYLE}</style>")?;
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

        let mut details = Vec::new();

        writeln!(out, "<div class=\"lock-session\">")?;

        let height = events.len();
        let style = format!("height: {height}em;");
        writeln!(
            out,
            r#"<div data-toggle="event-{lock}-details" class="timeline" style="{style}">"#
        )?;

        for (level, (id, name, open, backtrace, thread_index)) in events.into_iter().enumerate() {
            write_section(
                &mut out,
                level,
                Some(thread_index),
                id,
                (start, end),
                name,
                open,
                &children,
                &closes,
                backtrace,
                &mut details,
            )?;
        }

        writeln!(out, "</div>")?;

        if !details.is_empty() {
            writeln!(
                out,
                r#"<table id="event-{lock}-details" class="details" style="display: none;">"#
            )?;

            out.write_all(&details)?;
            writeln!(out, "</table>")?;
        }

        writeln!(out, "</div>")?;
        writeln!(out, "</div>")?;
    }

    writeln!(out, "</div>")?;
    writeln!(out, "<script type=\"text/javascript\">{SCRIPT}</script>")?;
    writeln!(out, "</body>")?;
    writeln!(out, "</html>")?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_section(
    out: &mut dyn io::Write,
    level: usize,
    thread_index: Option<usize>,
    id: EventId,
    span: (u64, u64),
    title: &str,
    open: u64,
    children: &HashMap<EventId, Vec<Child<'_>>>,
    closes: &HashMap<EventId, u64>,
    backtrace: Option<&EventBacktrace>,
    d: &mut Vec<u8>,
) -> io::Result<()> {
    let Some(close) = closes.get(&id).copied() else {
        return Ok(());
    };

    let (start, end) = span;
    let total = (end - start) as f32;

    let left = (((open - start) as f32 / total) * 100.0).round() as u32;
    let width = (((close - open) as f32 / total) * 100.0).round() as u32;

    let s = Duration::from_nanos(open);
    let e = Duration::from_nanos(close);
    let duration = Duration::from_nanos(close - open);

    if let Some(thread_index) = thread_index {
        writeln!(
            out,
            r#"<span class="section-heading" style="top: {level}em;"><span>{thread_index} / {id}</span></span>"#
        )?;
    }

    let style = format!("width: {width}%; left: {left}%; top: {level}em;");
    let hover_title = format!("{title} ({s:?}-{e:?})");

    writeln!(
        out,
        "<div id=\"event-{id}\" class=\"section {title}\" style=\"{style}\" title=\"{hover_title}\"></div>"
    )?;

    if let Some(thread_index) = thread_index {
        writeln! {
            d,
            r#"
            <tr>
                <td class="title" colspan="6">Thread: {thread_index} / Event: {id}</td>
            </tr>
            "#
        }?;
    }

    writeln! {
        d,
        r#"
        <tr>
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
        write_section(
            out, level, None, id, span, title, child_open, children, closes, backtrace, d,
        )?;
    }

    Ok(())
}
