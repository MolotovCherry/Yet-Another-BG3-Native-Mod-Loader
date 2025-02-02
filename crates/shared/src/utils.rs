use std::{
    ffi::c_void,
    sync::{Mutex, MutexGuard},
};

use windows::{core::Free, Win32::Foundation::HANDLE};

#[repr(transparent)]
#[derive(Debug, Default)]
pub struct OwnedHandle(HANDLE);

impl OwnedHandle {
    pub fn new(handle: HANDLE) -> Self {
        Self(handle)
    }

    /// Note: This HANDLE gets dropped at end of scope; it is POSSIBLE to keep a reference
    ///       to this since HANDLE: Copy
    pub fn as_raw_handle(&self) -> HANDLE {
        self.0
    }

    pub fn as_mut<U>(&mut self) -> *mut U {
        (self as *mut Self).cast()
    }
}

impl From<*mut c_void> for OwnedHandle {
    fn from(value: *mut c_void) -> Self {
        Self(HANDLE(value))
    }
}

impl From<HANDLE> for OwnedHandle {
    fn from(handle: HANDLE) -> Self {
        Self(handle)
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            self.0.free();
        }
    }
}

pub trait SuperLock<T> {
    fn super_lock(&self) -> MutexGuard<T>;
}

impl<T> SuperLock<T> for Mutex<T> {
    /// Always get a mutex guard regardless of poison status
    fn super_lock(&self) -> MutexGuard<T> {
        self.clear_poison();

        match self.lock() {
            Ok(v) => v,
            Err(_) => unreachable!("poison was cleared; this is impossible"),
        }
    }
}

/// Poor mans try {} blocks
#[macro_export]
macro_rules! tri {
    ($($code:tt)*) => {
        (|| {
            $(
                $code
            )*
        })()
    };
}
pub use tri;
