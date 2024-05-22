use std::{
    collections::HashMap,
    ffi::c_void,
    fs::{self, OpenOptions},
    os::windows::{fs::OpenOptionsExt as _, prelude::AsRawHandle},
};
use std::{mem, path::Path};

use eyre::{anyhow, bail, eyre, Context, Result};
use native_plugin_lib::{Plugin, Version};
use tracing::{error, info, trace, trace_span, warn};
use unicase::UniCase;
use windows::{
    core::{s, w, Error as WinError},
    Win32::{
        Foundation::{GetLastError, ERROR_INSUFFICIENT_BUFFER, HANDLE, HMODULE},
        Storage::FileSystem::{
            GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS,
            FILE_SHARE_READ,
        },
        System::{
            Diagnostics::Debug::WriteProcessMemory,
            LibraryLoader::{GetModuleHandleW, GetProcAddress},
            Memory::{VirtualAllocEx, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE},
            ProcessStatus::{EnumProcessModulesEx, GetModuleFileNameExW, LIST_MODULES_64BIT},
            Threading::{
                CreateRemoteThread, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION,
                PROCESS_VM_READ, PROCESS_VM_WRITE,
            },
        },
    },
};

use crate::{config::Config, helpers::OwnedHandle, paths::get_bg3_plugins_dir, popup::warn_popup};

#[allow(non_snake_case)]
pub fn inject_plugins(pid: u32, plugins_dir: &Path, config: &Config) -> Result<()> {
    let span = trace_span!("inject_plugins");
    let _guard = span.enter();

    // get loadlibraryw address as fn pointer
    let LoadLibraryW: unsafe extern "system" fn(*mut c_void) -> u32 = unsafe {
        mem::transmute(
            GetProcAddress(
                GetModuleHandleW(w!("kernel32")).context("Failed to get kernel32 module handle")?,
                s!("LoadLibraryW"),
            )
            .ok_or(WinError::from_win32())
            .context("Failed to get LoadLibraryW proc address")?,
        )
    };

    let handle: Result<OwnedHandle, _> = unsafe {
        OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
            false,
            pid,
        )
        .map(Into::into)
    };

    let Ok(handle) = handle else {
        error!(?handle, "failed to open process");
        warn_popup("Can't open process", format!("Failed to open the game process.\n\nThis could be due to a few reasons:\n1. when the program attempted to open the process, it was already gone\n2. you need higher permissions, e.g. admin perms to open it (try running this as admin)\n\nIf this isn't a perm problem, just try again\n\nError: {}", handle.unwrap_err()));
        return Ok(());
    };

    // checks if process has already had injection done on it
    let is_dirty = is_dirty(&handle);
    let Ok(is_dirty) = is_dirty else {
        error!(?is_dirty, "failed dirty check");
        warn_popup(
            "Failed process dirty check",
            format!(
                "Dirty check failed. Skipping process injection\n\n{}",
                is_dirty.unwrap_err()
            ),
        );
        return Ok(());
    };

    if is_dirty {
        // return ok as if nothing happened, however we will log this
        warn!("game process is dirty; aborting injection");
        return Ok(());
    }

    let read_dir = fs::read_dir(plugins_dir).context("Failed to read plugins_dir {plugins_dir}");
    let Ok(read_dir) = read_dir else {
        error!(?read_dir, "failed to read plugins dir");
        warn_popup(
            "Failed to read plugins dir",
            format!(
                "Attempted to read plugins dir {}, but failed opening it\n\nDo you have correct perms?\n\nError: {}",
                plugins_dir.display(),
                read_dir.unwrap_err()
            ),
        );

        return Ok(());
    };

    for entry in read_dir {
        let Ok(entry) = entry else {
            error!(?entry, "failed to read dir entry");
            warn_popup(
                "Failed to open path",
                format!(
                    "Attempted to read dir listing from {}, but a file inside could not be read\n\nError: {}",
                    plugins_dir.display(),
                    entry.unwrap_err()
                ),
            );

            return Ok(());
        };

        trace!("checking {entry:?} for injection plausability");

        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|e| e.to_ascii_lowercase() == "dll")
        {
            // check if plugin is disallowed or allowed
            let name = path
                .file_stem()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default();

            let contains_disabled = config
                .core
                .disabled
                .iter()
                .any(|s| UniCase::new(s) == UniCase::new(name));

            let data = native_plugin_lib::get_plugin_data(&path);
            let name = if let Ok(data) = data {
                let Plugin {
                    version:
                        Version {
                            major,
                            minor,
                            patch,
                        },
                    ..
                } = data;

                data.get_name()
                    .map(|n| format!("{n} v{major}.{minor}.{patch} ({name}.dll)"))
                    .unwrap_or(format!("{name}.dll v{major}.{minor}.{patch}"))
            } else {
                format!("{name}.dll")
            };

            if contains_disabled {
                info!("Skipping disabled plugin {name}");
                continue;
            }

            info!("Loading plugin {name}");

            let mut plugin_path = path
                .to_str()
                .ok_or(eyre!("Failed to convert plugin path"))?
                .encode_utf16()
                .collect::<Vec<_>>();
            plugin_path.push(b'\0' as u16);

            // 1 byte = u8, u16 = 2 bytes, len = number of elems in vector, so len * 2
            let plugin_path_len = plugin_path.len() * 2;

            let alloc_addr = unsafe {
                VirtualAllocEx(
                    handle.as_raw_handle(),
                    None,
                    plugin_path_len,
                    MEM_RESERVE | MEM_COMMIT,
                    PAGE_EXECUTE_READWRITE,
                )
            };

            if alloc_addr.is_null() {
                let error = unsafe { GetLastError() };
                error!(?error, "virtualallocex failed to allocate memory");
                warn_popup(
                    "Process injection failure",
                    format!("Failed to allocate memory in target process\n\nThis could be due to multiple reasons, but in any case, winapi returned an error that we cannot recover from. It's possible at this point that some plugins are injected, while others are not. Recommend restarting game and trying again\n\nError: {error:?}"),
                );
                return Ok(());
            }

            // Write the data to the process
            let write_res = unsafe {
                WriteProcessMemory(
                    handle.as_raw_handle(),
                    alloc_addr,
                    plugin_path.as_ptr() as *const _,
                    plugin_path_len,
                    None,
                )
            };
            if let Err(e) = write_res {
                error!(?e, "Failed to write to process");
                warn_popup(
                    "Process injection failure",
                    format!("Failed to write to process memory\n\nThis could be due to multiple reasons, but in any case, winapi returned an error that we cannot recover from. It's possible at this point that some plugins are injected, while others are not. Recommend restarting game and trying again\n\nError: {e}"),
                );
                return Ok(());
            }

            // start thread with dll
            // Note that the returned HANDLE is intentionally not closed!
            let rem_thread_res = unsafe {
                CreateRemoteThread(
                    handle.as_raw_handle(),
                    None,
                    0,
                    Some(LoadLibraryW),
                    Some(alloc_addr),
                    0,
                    None,
                )
            };
            if let Err(e) = rem_thread_res {
                error!(?e, "Failed to create remote thread");
                warn_popup(
                    "Process injection failure",
                    format!("Failed to create process remote thread\n\nThis could be due to multiple reasons, but in any case, winapi returned an error that we cannot recover from. It's possible at this point that some plugins are injected, while others are not. Recommend restarting game and trying again\n\nError: {e}"),
                );
                return Ok(());
            }
        }
    }

    Ok(())
}

// Determine whether the process has been tainted by previous dll injections
fn is_dirty(handle: &OwnedHandle) -> Result<bool> {
    let span = trace_span!("is_dirty");
    let _guard = span.enter();

    let (_, plugins_dir) = get_bg3_plugins_dir()?;

    let mut cache_id_map = HashMap::new();

    trace!(plugins_dir = %plugins_dir.display(), "checking dll path against dirs");

    let plugins_dir_id = dir_id(&plugins_dir).ok_or(anyhow!("failed to get id for plugins_dir"))?;
    cache_id_map.insert(plugins_dir.clone(), plugins_dir_id);

    let mut is_plugin = move |path: &str| {
        trace!("is_plugin_path checking dll @ {path}");

        // path
        //

        let path = Path::new(path);

        // not a dll file
        if !path.is_file()
            || !path
                .extension()
                .is_some_and(|ext| ext.to_ascii_lowercase() == "dll")
        {
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

fn dir_id(path: &Path) -> Option<u128> {
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

    let handle = dir.as_raw_handle();

    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    unsafe {
        GetFileInformationByHandle(HANDLE(handle as _), &mut info).ok()?;
    }

    let mut id = 0u128;
    id |= (info.dwVolumeSerialNumber as u128) << 64;
    id |= (info.nFileIndexHigh as u128) << 32;
    id |= info.nFileIndexLow as u128;

    trace!(id, path = %path.display(), "path id");

    Some(id)
}
