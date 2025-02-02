use std::{
    mem,
    panic::{self, AssertUnwindSafe},
    sync::Mutex,
};

use eyre::Result;
use shared::utils::SuperLock;
use tracing::{error, trace_span};
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM},
    UI::WindowsAndMessaging::EnumWindows,
};

type UserCallback<'a> = Box<dyn FnMut(HWND) -> Result<()> + Send + Sync + 'a>;

struct FfiCb(UserCallback<'static>);

trait StaticFfiCbMethods {
    unsafe fn set_cb(&self, cb: UserCallback);
    unsafe fn call(&self, hwnd: HWND) -> Result<()>;
    fn drop(&self);
}

impl StaticFfiCbMethods for Mutex<Option<FfiCb>> {
    /// SAFETY:
    /// Caller promises to drop before end of scope where original closure was created
    ///     this is because closure may have captures, this is !'static
    unsafe fn set_cb(&self, cb: UserCallback) {
        let _static = unsafe { mem::transmute::<UserCallback, UserCallback<'static>>(cb) };
        *self.super_lock() = Some(FfiCb(_static));
    }

    /// SAFETY:
    /// This must be called in scope where closure captures are still valid
    unsafe fn call(&self, hwnd: HWND) -> Result<()> {
        if let Some(cb) = &mut *self.super_lock() {
            cb.0(hwnd)
        } else {
            Ok(())
        }
    }

    fn drop(&self) {
        *self.super_lock() = None;
    }
}

static CB: Mutex<Option<FfiCb>> = Mutex::new(None);

#[allow(non_snake_case)]
pub fn EnumWindowsRs(cb: impl FnMut(HWND) -> Result<()> + Send + Sync) {
    let span = trace_span!("EnumWindowsRs");
    let _guard = span.enter();

    unsafe {
        CB.set_cb(Box::new(cb));
    }

    _ = unsafe { EnumWindows(Some(enum_cb), LPARAM(0)) };

    CB.drop();
}

extern "system" fn enum_cb(param0: HWND, _: LPARAM) -> BOOL {
    let span = trace_span!("enum_cb");
    let _guard = span.enter();

    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe { CB.call(param0) }));

    match result {
        // no panic and cb returned Ok
        Ok(Ok(_)) => true.into(),

        // no panic and callback returned Err
        Ok(Err(err)) => {
            error!("{err}");
            false.into()
        }

        // panic
        Err(e) => {
            // so it never panics
            mem::forget(e);
            false.into()
        }
    }
}
