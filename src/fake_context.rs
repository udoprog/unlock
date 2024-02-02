use crate::event::Event;

/// Indicate whether capture is enabled or not.
///
/// This is the fake version and will do nothing. To enable the real version,
/// set the `trace` feature.
#[inline(always)]
#[allow(unused)]
pub fn capture(enabled: bool) {
}

/// Drain the current capture of events.
///
/// This is the fake version and will always return an empty vector. To enable
/// the real version, set the `trace` feature.
#[inline(always)]
pub fn drain() -> Vec<Event> {
    Vec::new()
}
