use std::ffi::c_void;

use tracing::error;
use windows::Win32::{
    Foundation::{GetLastError, HLOCAL, LocalFree},
    Security::PSECURITY_DESCRIPTOR,
};

#[repr(transparent)]
pub struct PSecurityDescriptor(PSECURITY_DESCRIPTOR);

impl PSecurityDescriptor {
    pub fn as_mut(&mut self) -> *mut PSECURITY_DESCRIPTOR {
        &mut self.0
    }

    pub fn as_void(&self) -> *mut c_void {
        self.0.0
    }
}

impl From<PSECURITY_DESCRIPTOR> for PSecurityDescriptor {
    fn from(value: PSECURITY_DESCRIPTOR) -> Self {
        Self(value)
    }
}

impl Drop for PSecurityDescriptor {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            let res = unsafe { LocalFree(HLOCAL(self.0.0).into()) };
            if !res.is_invalid() {
                let err = unsafe { GetLastError() };
                error!(hlocal = ?res, ?err, "failed to free hlocal");
            }
        }
    }
}
