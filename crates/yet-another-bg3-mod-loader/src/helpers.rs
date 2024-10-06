use std::ffi::c_void;

use windows::{
    core::Free,
    Win32::Foundation::{HANDLE, HMODULE},
};

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

#[derive(Debug)]
pub struct OwnedModule(HMODULE);

impl OwnedModule {
    /// Note: This HMODULE gets dropped at end of scope; it is POSSIBLE to keep a reference
    ///       to this since HMODULE: Copy
    pub fn as_raw_module(&self) -> HMODULE {
        self.0
    }
}

impl From<HMODULE> for OwnedModule {
    fn from(value: HMODULE) -> Self {
        Self(value)
    }
}

impl Drop for OwnedModule {
    fn drop(&mut self) {
        unsafe {
            self.0.free();
        }
    }
}
