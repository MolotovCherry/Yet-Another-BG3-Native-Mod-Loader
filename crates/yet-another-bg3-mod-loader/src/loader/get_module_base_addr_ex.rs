use std::{
    ffi::OsString,
    os::windows::prelude::{OsStrExt as _, OsStringExt as _},
    path::Path,
};

use tracing::trace;
use windows::Win32::Foundation::HMODULE;

use super::{enum_modules::enum_modules, get_module_file_name_ex::get_module_file_name_ex_w};
use crate::helpers::OwnedHandle;

/// Note: This matches based on FULL path, not just the filename
#[allow(non_snake_case)]
pub fn GetModuleBaseEx<P: AsRef<Path>>(process: &OwnedHandle, module: P) -> Option<HMODULE> {
    let module = module.as_ref();
    trace!(module = %module.display(), "");

    let module = module.as_os_str().encode_wide().collect::<Vec<_>>();

    let module_name = OsString::from_wide(&module);
    let mut buf = vec![0u16; 1024];

    let mut entry = None;
    enum_modules(process, |module| {
        let path = {
            let path = get_module_file_name_ex_w(process, module, &mut buf)?;
            path.to_os_string()
        };

        trace!(path = %path.to_string_lossy(), "GetModuleBaseEx trying");

        if module_name == path {
            entry = Some(module);
            return Ok(false);
        }

        Ok(true)
    })
    .ok()?;

    entry
}
