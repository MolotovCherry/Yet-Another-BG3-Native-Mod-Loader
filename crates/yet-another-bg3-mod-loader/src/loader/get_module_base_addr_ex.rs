use std::{
    os::windows::ffi::{EncodeWide, OsStrExt as _},
    path::Path,
};

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
        eprintln!("{e}");
        return None;
    }

    let module = module.as_ref().as_os_str().encode_wide();

    loop {
        if module.is(&entry.szModule) {
            return Some(entry.hModule);
        }

        match unsafe { Module32NextW(snap, &mut entry) } {
            Ok(_) => continue,
            Err(e) if e.code() == ERROR_NO_MORE_FILES.to_hresult() => break,
            Err(e) => {
                eprintln!("{e}");
                break;
            }
        }
    }

    None
}

trait IsEqual {
    fn is(&self, str: &[u16]) -> bool;
}

impl IsEqual for EncodeWide<'_> {
    fn is(&self, str: &[u16]) -> bool {
        let mut iter = str.iter();
        let mut i = 0usize;

        while iter.next().is_some_and(|n| *n != 0) {
            i += 1;
        }

        let slice = &str[..i];

        let mut wide_iter = self.clone().zip(slice.iter().copied());
        loop {
            let Some((a, b)) = wide_iter.next() else {
                break;
            };

            if a != b {
                return false;
            }
        }

        true
    }
}
