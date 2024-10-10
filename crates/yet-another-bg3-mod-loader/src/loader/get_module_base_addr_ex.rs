use std::{os::windows::ffi::OsStrExt as _, path::Path};

use tracing::error;
use widestring::{U16CStr, U16Str};
use windows::Win32::{
    Foundation::{ERROR_NO_MORE_FILES, HMODULE},
    System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, MODULEENTRY32W, TH32CS_SNAPMODULE,
        TH32CS_SNAPMODULE32,
    },
};

#[allow(non_snake_case)]
pub fn GetModuleBaseEx<P: AsRef<Path>>(pid: u32, module: P) -> Option<HMODULE> {
    let snap =
        unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid).ok()? };

    let mut entry = MODULEENTRY32W {
        dwSize: size_of::<MODULEENTRY32W>() as _,
        ..Default::default()
    };

    if let Err(e) = unsafe { Module32FirstW(snap, &mut entry) } {
        error!("Module32FirstW: {e}");
        return None;
    }

    let module = module
        .as_ref()
        .as_os_str()
        .encode_wide()
        .collect::<Vec<_>>();

    let module = U16Str::from_slice(&module);

    loop {
        let sz_module = unsafe { U16CStr::from_ptr_str(entry.szModule.as_ptr()) };
        if module == sz_module {
            return Some(entry.hModule);
        }

        match unsafe { Module32NextW(snap, &mut entry) } {
            Ok(_) => continue,
            Err(e) if e.code() == ERROR_NO_MORE_FILES.to_hresult() => break,
            Err(e) => {
                error!("Module32FirstW: {e}");
                break;
            }
        }
    }

    None
}
