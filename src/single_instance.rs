use windows::{
    core::w,
    Win32::{Foundation::CloseHandle, System::Threading::CreateMutexW},
};

use crate::{helpers::OwnedHandle, popup::fatal_popup};

pub struct SingleInstance(OwnedHandle);

impl SingleInstance {
    /// Panics and shows error popup if another instance of app already running
    /// If it succeeds, then the app will be considered free to open again once this instance drops
    pub fn new() -> Self {
        let Ok(startup_mutex) =
            (unsafe { CreateMutexW(None, true, w!("yet-another-bg3-mod-loader")) })
        else {
            fatal_popup(
                "Already running",
                "Another instance of Yet Another Bg3 Mod Loader is already running",
            );
        };

        Self(startup_mutex.into())
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0.as_raw_handle());
        }
    }
}
