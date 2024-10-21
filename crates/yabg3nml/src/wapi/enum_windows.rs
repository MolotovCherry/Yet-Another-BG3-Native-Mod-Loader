use std::panic::{self, AssertUnwindSafe};

use eyre::Result;
use tracing::{error, trace_span};
use windows::Win32::{
    Foundation::{BOOL, FALSE, HWND, LPARAM, TRUE},
    UI::WindowsAndMessaging::EnumWindows,
};

type UserCallback<'a> = Box<dyn FnMut(HWND) -> Result<()> + Send + Sync + 'a>;

#[allow(non_snake_case)]
pub fn EnumWindowsRs(cb: impl FnMut(HWND) -> Result<()> + Send + Sync) {
    let span = trace_span!("EnumWindowsRs");
    let _guard = span.enter();

    let mut cb: UserCallback = Box::new(cb);
    _ = unsafe { EnumWindows(Some(enum_cb), LPARAM(&raw mut cb as isize)) };
}

extern "system" fn enum_cb(param0: HWND, param1: LPARAM) -> BOOL {
    let span = trace_span!("enum_cb");
    let _guard = span.enter();

    let cb = unsafe { &mut *(param1.0 as *mut UserCallback) };

    let result = panic::catch_unwind(AssertUnwindSafe(|| cb(param0)));

    match result {
        // no panic and cb returned Ok
        Ok(Ok(_)) => TRUE,

        // no panic and callback returned Err
        Ok(Err(err)) => {
            error!("{err}");
            FALSE
        }

        // panic
        Err(_) => FALSE,
    }
}
