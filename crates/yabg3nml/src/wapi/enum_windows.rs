use std::{
    mem,
    panic::{self, AssertUnwindSafe},
    ptr,
};

use eyre::Result;
use tracing::{error, trace_span};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM},
        UI::WindowsAndMessaging::EnumWindows,
    },
    core::BOOL,
};

type UserCallback<'a> = Box<dyn FnMut(HWND) -> Result<()> + Send + Sync + 'a>;

#[allow(non_snake_case)]
pub fn EnumWindowsRs(cb: impl FnMut(HWND) -> Result<()> + Send + Sync) {
    let span = trace_span!("EnumWindowsRs");
    let _guard = span.enter();

    let mut cb: UserCallback = Box::new(cb);
    _ = unsafe {
        EnumWindows(
            Some(enum_cb),
            LPARAM((&raw mut cb).expose_provenance() as _),
        )
    };
}

extern "system" fn enum_cb(param0: HWND, cb: LPARAM) -> BOOL {
    let span = trace_span!("enum_cb");
    let _guard = span.enter();

    let cb_raw: *mut UserCallback = ptr::with_exposed_provenance_mut(cb.0 as _);
    let cb = unsafe { &mut *cb_raw };

    let result = panic::catch_unwind(AssertUnwindSafe(|| cb(param0)));

    let res = match result {
        // no panic and cb returned Ok
        Ok(Ok(_)) => true,

        // no panic and callback returned Err
        Ok(Err(err)) => {
            error!("{err}");
            false
        }

        // panic
        Err(e) => {
            // so it never panics
            mem::forget(e);
            false
        }
    };

    res.into()
}
