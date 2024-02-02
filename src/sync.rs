use std::any::type_name;
use std::fmt;
use std::ops::{Deref, DerefMut};

use super::event::{EventId, LockId, LockKind};
use super::tracing_context::get;

/// Wrapper for `parking_lot::RwLock<T>`.
pub struct RwLock<T> {
    lock: LockId,
    inner: parking_lot::RwLock<T>,
}

impl<T> RwLock<T> {
    /// Create a new `RwLock<T>`.
    #[inline]
    pub fn new(value: T) -> Self {
        Self {
            lock: LockId::next(LockKind::RwLock),
            inner: parking_lot::RwLock::new(value),
        }
    }

    /// Lock the `RwLock<T>` for reading.
    #[inline]
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        let cx = get();
        let event = cx.enter(self.lock, "critical", type_name::<T>(), None);
        let inner = cx.with(self.lock, "read", type_name::<T>(), event, || {
            self.inner.read()
        });
        RwLockReadGuard { inner, event }
    }

    /// Lock the `RwLock<T>` for writing.
    #[inline]
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        let cx = get();
        let event = cx.enter(self.lock, "critical", type_name::<T>(), None);
        let inner = cx.with(self.lock, "write", type_name::<T>(), event, || {
            self.inner.write()
        });
        RwLockWriteGuard { inner, event }
    }
}

impl<T> fmt::Debug for RwLock<T>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// Wrapper for `parking_lot::RwLockReadGuard<T>`.
pub struct RwLockReadGuard<'a, T> {
    inner: parking_lot::RwLockReadGuard<'a, T>,
    event: Option<EventId>,
}

impl<'a, T> Deref for RwLockReadGuard<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T> Drop for RwLockReadGuard<'a, T> {
    #[inline]
    fn drop(&mut self) {
        get().leave(self.event);
    }
}

/// Wrapper for `parking_lot::RwLockWriteGuard<T>`.
pub struct RwLockWriteGuard<'a, T> {
    inner: parking_lot::RwLockWriteGuard<'a, T>,
    event: Option<EventId>,
}

impl<'a, T> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T> DerefMut for RwLockWriteGuard<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'a, T> Drop for RwLockWriteGuard<'a, T> {
    #[inline]
    fn drop(&mut self) {
        get().leave(self.event);
    }
}

/// Wrapper for `parking_lot::RwLock<T>`.
pub struct Mutex<T> {
    inner: parking_lot::Mutex<T>,
    lock: LockId,
}

impl<T> Mutex<T> {
    /// Create a new `Mutex<T>`.
    #[inline]
    pub fn new(value: T) -> Self {
        Self {
            inner: parking_lot::Mutex::new(value),
            lock: LockId::next(LockKind::Mutex),
        }
    }

    /// Lock the `Mutex<T>` for writing.
    #[inline]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        let cx = get();
        let event = cx.enter(self.lock, "critical", type_name::<T>(), None);
        let inner = cx.with(self.lock, "lock", type_name::<T>(), event, || {
            self.inner.lock()
        });
        MutexGuard { inner, event }
    }
}

impl<T> fmt::Debug for Mutex<T>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// Wrapper for `parking_lot::MutexGuard<T>`.
pub struct MutexGuard<'a, T> {
    inner: parking_lot::MutexGuard<'a, T>,
    event: Option<EventId>,
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    #[inline]
    fn drop(&mut self) {
        get().leave(self.event);
    }
}
