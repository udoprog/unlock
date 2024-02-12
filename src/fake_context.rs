use crate::event::Events;

/// Enable tracing.
///
/// This is the fake version and will do nothing. To enable the real version,
/// set the `trace` feature.
#[inline(always)]
#[allow(unused)]
pub fn capture() {}

/// Drain the current capture of events since the last time `capture` was
/// called.
///
/// This is the fake version and will always return an empty vector. To enable
/// the real version, set the `trace` feature.
#[inline(always)]
pub fn drain() -> Events {
    Events::new()
}
