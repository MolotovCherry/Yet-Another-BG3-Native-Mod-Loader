use std::path::{Path, PathBuf};

use eyre::Result;
use log::error;
use search_path::SearchPath;
use windows::{
    core::{PCSTR, PCWSTR},
    Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW},
};

use crate::{popup::fatal_popup, racy_cell::RacyCell};

static FUNCTION_PTRS: RacyCell<[*const (); NUM_FUNCTIONS]> =
    RacyCell::new([std::ptr::null(); NUM_FUNCTIONS]);

// Defines ORDINAL_BASE, NUM_FUNCTIONS, FUNCTION_NAMES, and asm
include!(concat!(env!("OUT_DIR"), "/func_defs.rs"));

// grabs correct dll from Path OR from same directory
fn get_libpath() -> PathBuf {
    static LIBNAME: &str = include_str!("../libname.cfg");
    let libname = LIBNAME.trim();

    let libname = libname.strip_suffix(".dll").unwrap_or(libname);
    let libname = format!("{libname}.dll");
    let path = Path::new(&libname);

    let search_path = SearchPath::new("Path").unwrap();
    let res = search_path.find_file(path);

    if let Some(res) = res {
        return res;
    }

    // search in same directory now
    if !path.exists() {
        fatal_popup(
            "Yet Another BG3 Mod Loader Error",
            format!("{libname}.dll not found"),
        );
    }

    path.to_path_buf()
}

pub fn load_proxy_fns() -> Result<()> {
    let libpath = get_libpath()
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();

    let lib = unsafe { LoadLibraryW(PCWSTR(libpath.as_ptr()))? };

    for (i, name) in FUNCTION_NAMES.iter().enumerate() {
        let name_ptr: *const u8 = if let Some(name) = name {
            name.as_ptr()
        } else {
            // MAKEINTRESOURCEA
            // https://github.com/microsoft/windows-rs/issues/641
            (i as u16 + ORDINAL_BASE) as *mut _
        };

        let addr = unsafe { GetProcAddress(lib, PCSTR(name_ptr)) };
        if let Some(fn_) = addr {
            unsafe {
                FUNCTION_PTRS.get_mut()[i] = fn_ as _;
            }
        } else {
            error!("Warning: unresolved import {name:?} at index {i}");
        }
    }

    Ok(())
}
