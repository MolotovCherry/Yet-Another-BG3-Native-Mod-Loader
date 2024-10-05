use std::os::windows::io::IntoRawHandle;
use std::path::Path;
use std::{collections::HashMap, fs::OpenOptions, os::windows::fs::OpenOptionsExt as _};

use eyre::{anyhow, bail, Result};
use shared::paths::get_bg3_plugins_dir;
use tracing::{error, trace, trace_span};
use windows::Win32::{
    Foundation::{GetLastError, ERROR_INSUFFICIENT_BUFFER, HMODULE},
    Storage::FileSystem::{
        FileIdInfo, GetFileInformationByHandleEx, FILE_FLAG_BACKUP_SEMANTICS, FILE_ID_INFO,
        FILE_SHARE_READ,
    },
    System::ProcessStatus::{EnumProcessModulesEx, GetModuleFileNameExW, LIST_MODULES_64BIT},
};

use crate::helpers::OwnedHandle;

#[derive(Debug, Copy, Clone, PartialEq)]
struct Id(u64, u128);

fn dir_id(path: &Path) -> Option<Id> {
    if !path.is_dir() {
        return None;
    }

    // abuse OpenOptions to call CreateFileW with the correct args to get a dir handle
    // this lets us avoid an unsafe call
    let dir = OpenOptions::new()
        .access_mode(0)
        .share_mode(FILE_SHARE_READ.0)
        .attributes(FILE_FLAG_BACKUP_SEMANTICS.0)
        // (self.create, self.truncate, self.create_new) {
        //    (false, false, false) => c::OPEN_EXISTING,
        .create(false)
        .truncate(false)
        .create_new(false)
        .open(path)
        .ok()?;

    let handle: OwnedHandle = dir.into_raw_handle().into();

    let mut info = FILE_ID_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            handle.as_raw_handle(),
            FileIdInfo,
            &mut info as *mut _ as *mut _,
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
        .ok()?;
    }

    let file_id = u128::from_le_bytes(info.FileId.Identifier);

    trace!(volume_id = info.VolumeSerialNumber, file_id, path = %path.display(), "dir id");

    Some(Id(info.VolumeSerialNumber, file_id))
}

// Determine whether the process has been tainted by previous dll injections
pub fn is_dirty(handle: &OwnedHandle) -> Result<bool> {
    let span = trace_span!("is_dirty");
    let _guard = span.enter();

    let plugins_dir = get_bg3_plugins_dir()?;

    let mut cache_id_map = HashMap::new();

    trace!(plugins_dir = %plugins_dir.display(), "checking dll path against dirs");

    let plugins_dir_id = dir_id(&plugins_dir).ok_or(anyhow!("failed to get id for plugins_dir"))?;
    cache_id_map.insert(
        plugins_dir.to_string_lossy().to_lowercase().into(),
        plugins_dir_id,
    );

    let mut is_plugin = move |path: &str| {
        let path = path.to_lowercase();
        let path = Path::new(&path);

        // not a dll file
        if !path.is_file() || !path.extension().is_some_and(|ext| ext == "dll") {
            return false;
        }

        // get parent dir
        let Some(parent) = path.parent() else {
            return false;
        };

        let id = if let Some(&id) = cache_id_map.get(parent) {
            id
        } else {
            let Some(path_id) = dir_id(parent) else {
                return false;
            };

            cache_id_map.insert(parent.to_path_buf(), path_id);

            path_id
        };

        plugins_dir_id == id
    };

    // fully initialize this in order to set len
    let mut modules: Vec<HMODULE> = vec![HMODULE::default(); 1024];
    let mut lpcbneeded = 0;

    let mut enum_proc_retries = 0;
    loop {
        let size = (modules.len() * std::mem::size_of::<HMODULE>()) as u32;

        let enum_res = unsafe {
            EnumProcessModulesEx(
                handle.as_raw_handle(),
                modules.as_mut_ptr(),
                size,
                &mut lpcbneeded,
                LIST_MODULES_64BIT,
            )
        };

        // sometimes it may fail, for example, during module changes and so on
        // so we may have to retry this once, but a few times just to be on the safe side
        // if it's still failing after, it's def an error
        //
        // 4 retries max
        if enum_res.is_err() && enum_proc_retries < 4 {
            error!(
                error = ?enum_res,
                retry = enum_proc_retries + 1,
                "EnumProcessModulesEx failed; retrying"
            );
            std::thread::sleep(std::time::Duration::from_millis(250));
            enum_proc_retries += 1;
            continue;
        }

        // reset since we made it past
        enum_proc_retries = 0;

        if let Err(e) = enum_res {
            error!(error = ?e, "EnumProcessModulesEx failed");
            bail!("EnumProcessModulesEx failed to check if process was dirty\n\nThis could happen for many reasons, among them:\n1. process disappeared during the call\n2. you don't have correct perms\n3. process is corrupted\n4. modules changed while we were reading\n\nPlease try again\n\nError: {e}");
        }

        trace!("Passed EnumProcessModulesEx check");

        // To determine if the lphModule array is too small to hold all module handles for the process,
        // compare the value returned in lpcbNeeded with the value specified in cb. If lpcbNeeded is greater
        // than cb, increase the size of the array and call EnumProcessModulesEx again.
        if lpcbneeded > size {
            trace!("lpcbneeded {lpcbneeded} > size {size}; increasing +1024");
            modules.resize(modules.len() + 1024, HMODULE::default());
            continue;
        }

        // IMPORTANT
        // Do not call CloseHandle on any of the handles returned by this function. The information comes from a
        // snapshot, so there are no resources to be freed.

        break;
    }

    // To determine how many modules were enumerated by the call to EnumProcessModulesEx, divide the resulting
    // value in the lpcbNeeded parameter by sizeof(HMODULE).
    let n_modules = lpcbneeded as usize / std::mem::size_of::<HMODULE>();
    let modules = &modules[..n_modules];

    let mut name = vec![0u16; 1024];
    let mut total_retries = 0;
    'for_modules: for &module in modules {
        let mut retry = 0;
        let len = loop {
            let len = unsafe { GetModuleFileNameExW(handle.as_raw_handle(), module, &mut name) };

            // If the buffer is too small to hold the module name, the string is truncated to nSize characters including the
            // terminating null character, the function returns nSize, and the function sets the last error to ERROR_INSUFFICIENT_BUFFER.
            // If the function fails, the return value is 0 (zero). To get extended error information, call GetLastError.
            if len == name.len() as u32 || len == 0 {
                let error = unsafe { GetLastError() };

                if error.is_ok() {
                    break len;
                }

                if error == ERROR_INSUFFICIENT_BUFFER {
                    trace!("ERROR_INSUFFICIENT_BUFFER, increasing +1024");
                    name.resize(name.len() + 1024, 0u16);

                    continue;
                }

                error!(
                    handle = ?handle.as_raw_handle(),
                    module = ?module,
                    error = ?error,
                    retry = retry,
                    total_retries = total_retries,
                    "failed to open module handle",
                );

                if total_retries > 9 {
                    bail!("GetModuleFileNameExW is failing too much. This could signify the process is already gone or in a corrupted state. Please try again");
                }

                if retry < 3 {
                    retry += 1;
                    total_retries += 1;
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    continue;
                }

                continue 'for_modules;
            }

            break len;
        };

        let path = String::from_utf16_lossy(&name[..len as usize]);

        trace!("found loaded module @ {path}");

        if is_plugin(&path) {
            return Ok(true);
        }
    }

    Ok(false)
}
