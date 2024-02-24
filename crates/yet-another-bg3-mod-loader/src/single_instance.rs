use tracing::trace;
use windows::{
    core::w,
    Win32::{
        Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS},
        System::Threading::CreateMutexW,
    },
};

use crate::{helpers::OwnedHandle, popup::fatal_popup};

pub struct SingleInstance(OwnedHandle);

impl SingleInstance {
    /// Panics and shows error popup if another instance of app already running
    /// If it succeeds, then the app will be considered free to open again once this instance drops
    pub fn new() -> Self {
        let mutex = unsafe { CreateMutexW(None, true, w!("yet-another-bg3-mod-loader")).unwrap() };

        if let Err(e) = unsafe { GetLastError() } {
            if e.code() == ERROR_ALREADY_EXISTS.into() {
                fatal_popup(
                    "Yet Another Bg3 Mod Loader",
                    "Another instance is already running",
                );
            }

            trace!("CreateMutexW failed, but we have fallthrough: {e:?}");
        }

        Self(mutex.into())
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0.as_raw_handle());
        }
    }
}
