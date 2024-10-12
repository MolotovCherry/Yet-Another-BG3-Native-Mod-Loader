use std::{os::windows::prelude::OsStrExt as _, path::Path};

use tracing::{error, trace};
use widestring::U16Str;
use windows::Win32::Foundation::HMODULE;

use super::{enum_modules::enum_modules, get_module_file_name_ex::get_module_file_name_ex_w};
use crate::helpers::OwnedHandle;

/// Note: This matches based on FULL path, not just the filename
#[allow(non_snake_case)]
pub fn GetModuleBaseEx<P: AsRef<Path>>(process: &OwnedHandle, module: P) -> Option<HMODULE> {
    let module = module.as_ref();
    trace!(module = %module.display(), "GetModuleBaseEx checking for");

    let module = module.as_os_str().encode_wide().collect::<Vec<_>>();
    let module_name = U16Str::from_slice(&module);

    let mut buf = vec![0u16; 1024];
    let mut entry = None;
    let res = enum_modules(process, |module| {
        let path = get_module_file_name_ex_w(process, Some(module), &mut buf)?;

        trace!(path = %path.to_string_lossy(), "GetModuleBaseEx trying");

        if module_name == path {
            entry = Some(module);
            return Ok(false);
        }

        Ok(true)
    });

    if let Err(e) = res {
        error!(%e, path = %module_name.display(), "GetModuleBaseEx: error looking for module");
    }

    entry
}
