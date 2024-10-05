use eyre::Result;
use windows::{
    core::w,
    Win32::{Foundation::ERROR_ALREADY_EXISTS, System::Threading::CreateMutexW},
};

use crate::{helpers::OwnedHandle, popup::fatal_popup};

#[allow(unused)]
pub struct SingleInstance(OwnedHandle);

impl SingleInstance {
    /// Panics and shows error popup if another instance of app already running
    /// If it succeeds, then the app will be considered free to open again once this instance drops
    pub fn new() -> Result<Self> {
        let mutex = unsafe { CreateMutexW(None, true, w!("yet-another-bg3-mod-loader")) };

        match mutex {
            Ok(v) => Ok(Self(v.into())),

            Err(e) => {
                let message = if e.code() == ERROR_ALREADY_EXISTS.to_hresult() {
                    "Another instance is already running"
                } else {
                    &format!("CreateMutexW failure: {e}")
                };

                fatal_popup("Yet Another Bg3 Mod Loader", message);
            }
        }
    }
}
