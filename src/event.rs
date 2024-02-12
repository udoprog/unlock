#[cfg(feature = "trace")]
use std::backtrace::{Backtrace, BacktraceStatus};
use std::borrow::Cow;
use std::fmt;
use std::num::{NonZeroU32, NonZeroUsize};
#[cfg(feature = "trace")]
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

const LOCK_ID_MASK: u32 = 0x3FFFFFFF;
const LOCK_KIND_SHIFT: u32 = 30;

#[derive(Debug)]
#[repr(u32)]
pub(super) enum LockKind {
    RwLock = 1,
    Mutex = 2,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
#[repr(transparent)]
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

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
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

/// A backtrace that can be serialized.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
pub struct EventBacktrace(Box<str>);

impl EventBacktrace {
    #[cfg(feature = "trace")]
    pub(super) fn from_capture(backtrace: Backtrace) -> Option<Self> {
        match backtrace.status() {
            BacktraceStatus::Captured => Some(Self(format!("{}", backtrace).into())),
            _ => None,
        }
    }
}

impl fmt::Display for EventBacktrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A recorded opening event.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Event {
    /// The unique identifier of this event.
    pub(super) id: EventId,
    /// Nanoseconds since tracing started.
    pub(super) timestamp: u64,
    /// The index of the thread the event was recorded on.
    pub(super) thread_index: usize,
    /// The parent event this event is a child of.
    pub(super) parent: Option<EventId>,
    /// The name of the event.
    pub(super) name: Cow<'static, str>,
    /// The type name which is wrapped in the lock.
    pub(super) type_name: Cow<'static, str>,
    /// The unique sequential identifier and kind of the lock.
    pub(super) lock: LockId,
    /// Capture backtrace if RUST_BACKTRACE=1 or RUST_LIB_BACKTRACE=1 is
    /// set.
    #[cfg_attr(feature = "serde", serde(default))]
    pub(super) backtrace: Option<EventBacktrace>,
}

/// A recorded leaving event.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Leave {
    /// Event emitted when a particular section has been left.
    ///
    /// The `sibling` identifier is the identifier of the matching event that
    /// opened this section.
    pub(super) sibling: EventId,
    /// Thread index.
    pub(super) thread_index: usize,
    /// The timestamp when the event was left.
    pub(super) timestamp: u64,
}

/// Collection of collected events.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Events {
    pub(super) enters: Vec<Event>,
    pub(super) leaves: Vec<Leave>,
}

impl Events {
    /// The number of enter events in the collection.
    pub fn len(&self) -> usize {
        self.enters.len()
    }

    /// Test if the collection of events is empty.
    pub fn is_empty(&self) -> bool {
        self.enters.is_empty()
    }

    pub(super) fn new() -> Self {
        Self {
            enters: Vec::new(),
            leaves: Vec::new(),
        }
    }
}
