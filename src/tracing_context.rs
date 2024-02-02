use std::backtrace::Backtrace;
use std::cell::Cell;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::sync::Once;
use std::time::Instant;

use crate::event::{Event, EventBacktrace, EventId, EventKind, LockId};

/// Indicate whether capture is enabled or not.
pub fn capture(enabled: bool) {
    get().capture(enabled);
}

/// Drain the current capture of events.
pub fn drain() -> Vec<Event> {
    get().drain()
}

static mut TRACING_CONTEXT: NonNull<TracingContext> = NonNull::dangling();
static INIT_TRACING_CONTEXT: Once = Once::new();

/// Rotating statically known index of the current thread.
static THREAD_INDEX: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static THREAD_INDEX_THREAD: Cell<Option<usize>> = Cell::new(None);
}

/// Access the global tracing context.
pub(super) fn get() -> &'static TracingContext {
    unsafe {
        INIT_TRACING_CONTEXT.call_once(|| {
            TRACING_CONTEXT =
                NonNull::from(Box::leak(Box::new(TracingContext::new(num_cpus::get()))));
        });
        TRACING_CONTEXT.as_ref()
    }
}

/// A context capturing tracing events.
pub(super) struct TracingContext {
    // Whether capture is enabled or not.
    capture: AtomicBool,
    // shaded storage for events to minimize contention.
    events: Vec<Mutex<Vec<Event>>>,
    // The instant tracing was started.
    start: Instant,
}

impl TracingContext {
    /// Create a new tracing context.
    pub(super) fn new(threads: usize) -> Self {
        let mut events = Vec::with_capacity(threads);

        for _ in 0..threads.max(1) {
            events.push(Mutex::new(Vec::new()));
        }

        Self {
            capture: AtomicBool::new(false),
            events,
            start: Instant::now(),
        }
    }

    /// Set whether capture is enabled.
    pub(super) fn capture(&self, enabled: bool) {
        self.capture.store(enabled, Ordering::Release);
    }

    /// Enter the given span.
    pub(super) fn enter(
        &self,
        lock: LockId,
        name: &'static str,
        type_name: &'static str,
        parent: Option<EventId>,
    ) -> Option<EventId> {
        if !self.capture.load(Ordering::Acquire) {
            return None;
        }

        let id = self.record(EventKind::Enter {
            parent,
            name: name.into(),
            type_name: type_name.into(),
            lock,
            backtrace: EventBacktrace::from_capture(Backtrace::capture()),
        });

        Some(id)
    }

    /// Leave the given span.
    pub(super) fn leave(&self, sibling: Option<EventId>) {
        if self.capture.load(Ordering::Acquire) {
            self.record(EventKind::Leave { sibling });
        }
    }

    /// Record events around the given closure.
    pub(super) fn with<F, T>(
        &self,
        lock: LockId,
        name: &'static str,
        type_name: &'static str,
        parent: Option<EventId>,
        f: F,
    ) -> T
    where
        F: FnOnce() -> T,
    {
        if !self.capture.load(Ordering::Acquire) {
            return f();
        }

        let id = self.record(EventKind::Enter {
            parent,
            name: name.into(),
            type_name: type_name.into(),
            lock,
            backtrace: EventBacktrace::from_capture(Backtrace::capture()),
        });

        let result = f();
        self.record(EventKind::Leave { sibling: Some(id) });
        result
    }

    /// Record an event.
    fn record(&self, kind: EventKind) -> EventId {
        // NB: This is at risk of being truncated, but that still gives us ~584
        // years worth of tracing.
        let duration = Instant::now().duration_since(self.start).as_nanos() as u64;

        let index = THREAD_INDEX_THREAD.with(|index| {
            if let Some(index) = index.get() {
                return index;
            }

            let new_index = THREAD_INDEX.fetch_add(1, Ordering::Relaxed);
            index.set(Some(new_index));
            new_index
        });

        let id = EventId::next();
        let mut events = self.events[index % self.events.len()].lock().unwrap();
        events.push(Event::new(id, duration, index, kind));
        id
    }

    /// Drain events.
    ///
    /// If capture is enabled while draining, the exact events recorded are
    /// not specified.
    pub(super) fn drain(&self) -> Vec<Event> {
        let mut output = Vec::new();

        for event in self.events.iter() {
            let mut events = event.lock().unwrap();
            output.extend(events.drain(..));
        }

        output.sort_by_key(|event| event.id);
        output
    }
}
