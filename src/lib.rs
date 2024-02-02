//! [<img alt="github" src="https://img.shields.io/badge/github-udoprog/unlock-8da0cb?style=for-the-badge&logo=github" height="20">](https://github.com/udoprog/unlock)
//! [<img alt="crates.io" src="https://img.shields.io/crates/v/unlock.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/unlock)
//! [<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-unlock-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/unlock)
//!
//! Helpers for tracing and troubleshooting multithreaded code.
//!
//! ![Example Trace](https://github.com/udoprog/unlock/blob/main/trace.png?raw=true)
//!
//! <br>
//!
//! ## Usage
//!
//! Import `RwLock` and `Mutex` from this crate instead of `parking_lot` directly.
//!
//! After this, you can instrument a section of code like this:
//!
//! ```no_run
//! let condition = true;
//!
//! if condition {
//!     unlock::capture();
//! }
//!
//! /* do some work */
//!
//! if condition {
//!     let events = unlock::drain();
//!
//!     let f = std::fs::File::create("trace.html")?;
//!     unlock::html::write(f, &events)?;
//!     println!("Wrote trace.html");
//! }
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```
//!
//! <br>
//!
//! ## How does it work
//!
//! This library provides two facade types:
//! * [`RwLock`]
//! * [`Mutex`]
//!
//! These integrate with a high performance concurrent tracing system to capture
//! events. While this will have some overhead, we aim to make it as small as
//! possible.
//!
//! Once a workload has been instrumented, the `drain` function can be called to
//! collect these events, which then can be formatted using either built-in
//! methods such as [`html::write`], or serialized as you please using `serde`
//! for processing later.
//!
//! [`RwLock`]: https://docs.rs/unlock/latest/unlock/type.RwLock.html
//! [`Mutex`]: https://docs.rs/unlock/latest/unlock/type.Mutex.html
//! [`html::write`]: https://docs.rs/unlock/latest/unlock/html/fn.write.html

mod event;
pub use self::event::Event;

#[cfg(feature = "trace")]
mod sync;
#[doc(inline)]
#[cfg(feature = "trace")]
pub use self::sync::*;

#[cfg_attr(feature = "trace", path = "tracing_context.rs")]
#[cfg_attr(not(feature = "trace"), path = "fake_context.rs")]
mod tracing_context;

pub use self::tracing_context::{capture, drain};

pub mod html;

#[cfg(not(feature = "trace"))]
pub use parking_lot::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
