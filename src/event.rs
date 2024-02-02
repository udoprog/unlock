#[cfg(feature = "trace")]
use std::backtrace::{Backtrace, BacktraceStatus};
use std::borrow::Cow;
use std::fmt;
use std::num::{NonZeroU32, NonZeroUsize};
#[cfg(feature = "trace")]
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

const LOCK_ID_MASK: u32 = 0x3FFFFFFF;
const LOCK_KIND_SHIFT: u32 = 30;

#[derive(Debug)]
#[repr(u32)]
pub(super) enum LockKind {
    RwLock = 1,
    Mutex = 2,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
#[serde(transparent)]
pub(super) struct LockId(NonZeroU32);

impl LockId {
    /// Create a new unique identifier.
    #[cfg(feature = "trace")]
    pub(super) fn next(kind: LockKind) -> Self {
        static LOCK_ID: AtomicU32 = AtomicU32::new(1);

        loop {
            if let Some(id) = NonZeroU32::new(LOCK_ID.fetch_add(1, Ordering::Relaxed)) {
                assert!(LOCK_ID_MASK >= id.get(), "wgpu-sync: Too many locks");
                return Self(((kind as u32) << LOCK_KIND_SHIFT) | id);
            }
        }
    }

    /// Get the index of this lock.
    pub(super) fn index(self) -> usize {
        (self.0.get() & LOCK_ID_MASK) as usize
    }

    /// Get the kind of lock this is.
    pub(super) fn kind(self) -> LockKind {
        match self.0.get() >> LOCK_KIND_SHIFT {
            1 => LockKind::RwLock,
            2 => LockKind::Mutex,
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for LockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for LockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("LockId")
            .field(&self.kind())
            .field(&self.index())
            .finish()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[serde(transparent)]
pub(super) struct EventId(NonZeroUsize);

impl EventId {
    /// Create a new unique identifier.
    #[cfg(feature = "trace")]
    pub(super) fn next() -> Self {
        // Provides a total ordering to events recorded. Note that this is not
        // guaranteed to be a globally observable order.
        static EVENT_ID: AtomicUsize = AtomicUsize::new(1);

        if let Some(id) = NonZeroUsize::new(EVENT_ID.fetch_add(1, Ordering::Relaxed)) {
            return Self(id);
        }

        panic!("wgpu-sync: Too many events")
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) enum EventKind {
    /// Event emitted when a particular named section has been entered.
    Enter {
        /// The parent event this event is a child of.
        parent: Option<EventId>,
        /// The name of the event.
        name: Cow<'static, str>,
        /// The type name which is wrapped in the lock.
        type_name: Cow<'static, str>,
        /// The unique sequential identifier and kind of the lock.
        lock: LockId,
        /// Capture backtrace if RUST_BACKTRACE=1 or RUST_LIB_BACKTRACE=1 is
        /// set.
        #[serde(default)]
        backtrace: Option<EventBacktrace>,
    },
    /// Event emitted when a particular section has been left.
    ///
    /// The `sibling` identifier is the identifier of the matching event that
    /// opened this section.
    Leave { sibling: Option<EventId> },
}

/// A backtrace that can be serialized.
#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct EventBacktrace(String);

impl EventBacktrace {
    #[cfg(feature = "trace")]
    pub(super) fn from_capture(backtrace: Backtrace) -> Option<Self> {
        match backtrace.status() {
            BacktraceStatus::Captured => Some(Self(format!("{}", backtrace))),
            _ => None,
        }
    }
}

impl fmt::Display for EventBacktrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A recorded event.
#[derive(Serialize, Deserialize, Debug)]
pub struct Event {
    /// The unique identifier of this event.
    pub(super) id: EventId,
    /// Nanoseconds since tracing started.
    pub(super) timestamp: u64,
    /// The index of the thread the event was recorded on.
    pub(super) thread_index: usize,
    /// The kind of the event.
    pub(super) kind: EventKind,
}

impl Event {
    #[cfg(feature = "trace")]
    pub(super) fn new(id: EventId, timestamp: u64, thread_index: usize, kind: EventKind) -> Self {
        Self {
            id,
            timestamp,
            thread_index,
            kind,
        }
    }
}
