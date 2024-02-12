use std::backtrace::Backtrace;
use std::cell::Cell;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Once;
use std::time::Instant;

use parking_lot::Mutex;

use crate::event::{Event, EventBacktrace, EventId, Events, Leave, LockId};

/// Initial event capacity for each thread.
const CAPACITY: usize = 8192;

/// Configure whether capturing is enabled or not.
///
/// This can be used to enable capture in detail for particular sections of
/// code.
///
/// Once called capturing will be started and the timestamp for the capture
/// system will be reset.
pub fn capture() {
    get().capture();
}

/// Disable capture and drain the current collection of events.
pub fn drain() -> Events {
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

struct ThreadStorage {
    enters: Vec<Event>,
    leaves: Vec<Leave>,
}

/// A context capturing tracing events.
pub(super) struct TracingContext {
    // shaded storage for events to minimize contention.
    storage: Vec<Mutex<ThreadStorage>>,
    // The instant tracing was started.
    start: Instant,
    // Once capturing is started, this will be set to the instant it was started
    // so that timestamps can be adjusted relative to it.
    adjust: AtomicU64,
}

impl TracingContext {
    /// Create a new tracing context.
    pub(super) fn new(threads: usize) -> Self {
        let mut storage = Vec::with_capacity(threads);

        for _ in 0..threads.max(1) {
            storage.push(Mutex::new(ThreadStorage {
                enters: Vec::with_capacity(CAPACITY),
                leaves: Vec::with_capacity(CAPACITY),
            }));
        }

        Self {
            storage,
            start: Instant::now(),
            adjust: AtomicU64::new(u64::MAX),
        }
    }

    /// Set whether capture is enabled.
    pub(super) fn capture(&self) {
        self.adjust.store(
            Instant::now().duration_since(self.start).as_nanos() as u64,
            Ordering::Release,
        );
    }

    /// Enter the given span.
    pub(super) fn enter(
        &self,
        lock: LockId,
        name: &'static str,
        type_name: &'static str,
        parent: Option<EventId>,
    ) -> Option<EventId> {
        if self.adjust.load(Ordering::Acquire) == u64::MAX {
            return None;
        }

        let id = EventId::next();
        let name = name.into();
        let type_name = type_name.into();
        let backtrace = EventBacktrace::from_capture(Backtrace::capture());

        self.record(|storage, thread_index, timestamp| {
            storage.enters.push(Event {
                id,
                timestamp,
                thread_index,
                parent,
                name,
                type_name,
                lock,
                backtrace,
            })
        });

        Some(id)
    }

    /// Leave the given span.
    pub(super) fn leave(&self, sibling: Option<EventId>) {
        if let Some(sibling) = sibling {
            self.record(|storage, thread_index, timestamp| {
                storage.leaves.push(Leave {
                    sibling,
                    thread_index,
                    timestamp,
                })
            });
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
        if self.adjust.load(Ordering::Acquire) == u64::MAX {
            return f();
        }

        let id = EventId::next();
        let name = name.into();
        let type_name = type_name.into();
        let backtrace = EventBacktrace::from_capture(Backtrace::capture());

        self.record(|storage, thread_index, timestamp| {
            storage.enters.push(Event {
                id,
                timestamp,
                thread_index,
                parent,
                name,
                type_name,
                lock,
                backtrace,
            })
        });

        let result = f();

        self.record(|storage, thread_index, timestamp| {
            storage.leaves.push(Leave {
                thread_index,
                sibling: id,
                timestamp,
            })
        });

        result
    }

    /// Record an event.
    fn record<F>(&self, f: F)
    where
        F: FnOnce(&mut ThreadStorage, usize, u64),
    {
        let thread_index = thread_index();
        // NB: This is at risk of being truncated, but that still gives us ~584
        // years worth of tracing.
        let duration = Instant::now().duration_since(self.start).as_nanos() as u64;
        f(
            &mut self.storage[thread_index % self.storage.len()].lock(),
            thread_index,
            duration,
        );
    }

    /// Drain events.
    ///
    /// If capture is enabled while draining, the exact events recorded are
    /// not specified.
    pub(super) fn drain(&self) -> Events {
        let adjust = self.adjust.swap(u64::MAX, Ordering::AcqRel);

        if adjust == u64::MAX {
            return Events::new();
        }

        let mut events = Events::new();

        for storage in self.storage.iter() {
            let mut storage = storage.lock();

            for mut enter in storage.enters.drain(..) {
                enter.timestamp -= adjust;
                events.enters.push(enter);
            }

            for mut leave in storage.leaves.drain(..) {
                leave.timestamp -= adjust;
                events.leaves.push(leave);
            }
        }

        events.enters.sort_by_key(|event| event.id);
        events.leaves.sort_by_key(|event| event.sibling);
        events
    }
}

fn thread_index() -> usize {
    THREAD_INDEX_THREAD.with(|index| {
        if let Some(index) = index.get() {
            return index;
        }

        let new_index = THREAD_INDEX.fetch_add(1, Ordering::Relaxed);
        index.set(Some(new_index));
        new_index
    })
}
