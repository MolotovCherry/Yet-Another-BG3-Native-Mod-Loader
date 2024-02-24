use windows::Win32::Foundation::{CloseHandle, HANDLE};

#[derive(Debug)]
pub struct OwnedHandle(HANDLE);

impl OwnedHandle {
    /// Note: This HANDLE gets dropped at end of scope; it is POSSIBLE to keep a reference
    ///       to this since HANDLE: Copy
    pub fn as_raw_handle(&self) -> HANDLE {
        self.0
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
            let _ = CloseHandle(self.0);
        }
    }
}
