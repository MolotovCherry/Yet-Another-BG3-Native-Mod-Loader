use std::ptr;

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM},
        UI::WindowsAndMessaging::EnumWindows,
    },
    core::BOOL,
};

type Handles = Vec<HWND>;

#[allow(non_snake_case)]
pub fn EnumWindowsRs() -> Handles {
    let mut v = Handles::new();

    #[rustfmt::skip]
    let _ = unsafe {
        EnumWindows(
            Some(enum_cb),
            LPARAM(
                (&raw mut v).expose_provenance() as _
            )
        )
    };

    v
}

extern "system" fn enum_cb(param0: HWND, v: LPARAM) -> BOOL {
    let v = ptr::with_exposed_provenance_mut::<Handles>(v.0 as _);
    let handles = unsafe { &mut *v };
    handles.push(param0);
    true.into()
}
