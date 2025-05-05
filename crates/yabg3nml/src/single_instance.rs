use shared::{popup::fatal_popup, utils::OwnedHandle};
use windows::{
    Win32::{
        Foundation::{ERROR_ALREADY_EXISTS, GetLastError},
        System::Threading::CreateMutexW,
    },
    core::w,
};

#[allow(unused)]
pub struct SingleInstance(OwnedHandle);

impl SingleInstance {
    /// Panics and shows error popup if another instance of app already running
    /// If it succeeds, then the app will be considered fee to open again once this instance drops
    pub fn new() -> Self {
        let mutex = unsafe { CreateMutexW(None, true, w!(r"yet-another-bg3-native-mod-loader")) };

        let handle = match mutex {
            Ok(h) => unsafe { OwnedHandle::new(h) },
            Err(e) => {
                fatal_popup(
                    "Yet Another BG3 Native Mod Loader",
                    format!("mutex failed: {e}"),
                );
            }
        };

        match unsafe { GetLastError() } {
            e if e == ERROR_ALREADY_EXISTS => {
                fatal_popup(
                    "Yet Another BG3 Native Mod Loader",
                    "Another instance is already running",
                );
            }

            e if e.is_err() => {
                fatal_popup(
                    "Yet Another BG3 Native Mod Loader",
                    format!("CreateMutexW failure: {e:?}"),
                );
            }

            _ => (),
        }

        Self(handle)
    }
}
