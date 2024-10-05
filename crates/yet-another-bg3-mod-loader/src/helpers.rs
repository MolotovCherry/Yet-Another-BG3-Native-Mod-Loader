use std::{ffi::c_void, os::windows::ffi::EncodeWide};

use windows::{core::Free, Win32::Foundation::HANDLE};

#[derive(Debug)]
pub struct OwnedHandle(HANDLE);

impl OwnedHandle {
    /// Note: This HANDLE gets dropped at end of scope; it is POSSIBLE to keep a reference
    ///       to this since HANDLE: Copy
    pub fn as_raw_handle(&self) -> HANDLE {
        self.0
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

impl From<OwnedHandle> for HANDLE {
    fn from(handle: OwnedHandle) -> Self {
        handle.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            self.0.free();
        }
    }
}

pub trait IsEqual {
    fn is(&self, str: &[u16]) -> bool;
}

impl IsEqual for EncodeWide<'_> {
    fn is(&self, str: &[u16]) -> bool {
        let mut iter = str.iter();
        let mut i = 0usize;

        while iter.next().is_some_and(|n| *n != 0) {
            i += 1;
        }

        let slice = &str[..i];

        let mut wide_iter = self.clone().zip(slice.iter().copied());
        loop {
            let Some((a, b)) = wide_iter.next() else {
                break;
            };

            if a != b {
                return false;
            }
        }

        true
    }
}
