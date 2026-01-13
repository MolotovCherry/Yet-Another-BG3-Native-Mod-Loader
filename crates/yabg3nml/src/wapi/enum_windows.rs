use std::{mem, sync::LazyLock};

use sayuri::sync::Mutex;
use shared::utils::ThreadedWrapper;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM},
        UI::WindowsAndMessaging::EnumWindows,
    },
    core::BOOL,
};

static HANDLES: LazyLock<Mutex<Vec<ThreadedWrapper<HWND>>>> = LazyLock::new(Mutex::default);

#[allow(non_snake_case)]
pub fn EnumWindowsRs() -> Vec<HWND> {
    _ = unsafe { EnumWindows(Some(enum_cb), LPARAM(0)) };

    mem::take(&mut *HANDLES.lock())
        .into_iter()
        .map(ThreadedWrapper::into_inner)
        .collect()
}

extern "system" fn enum_cb(param0: HWND, _: LPARAM) -> BOOL {
    HANDLES.lock().push(unsafe { ThreadedWrapper::new(param0) });
    true.into()
}
