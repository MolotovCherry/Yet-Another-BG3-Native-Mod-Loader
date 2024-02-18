use std::{ffi::c_void, fs};
use std::{mem, path::Path};

use anyhow::{anyhow, Context, Result};
use bg3_plugin_lib::{Plugin, Version};
use log::{debug, info, warn};
use windows::{
    core::{s, w, Error as WinError},
    Win32::{
        Foundation::{GetLastError, ERROR_INSUFFICIENT_BUFFER, HMODULE},
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

use crate::{config::Config, helpers::OwnedHandle, paths::get_bg3_plugins_dir};

#[allow(non_snake_case)]
pub fn inject_plugins(pid: u32, plugins_dir: &Path, config: &Config) -> anyhow::Result<()> {
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

    let handle: OwnedHandle = unsafe {
        OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
            false,
            pid,
        )
        .context("Failed to OpenProcess for {pid}")?
        .into()
    };

    // checks if process has already had injection done on it
    if is_dirty(&handle, config)? {
        // return ok as if nothing happened, however we will log this
        warn!("game process already dirty; aborting injection");
        return Ok(());
    }

    for entry in fs::read_dir(plugins_dir).context("Failed to read plugins_dir {plugins_dir}")? {
        let entry = entry?;

        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|e| e == "dll") {
            // check if plugin is disallowed or allowed
            let name = path
                .file_stem()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default();

            let data = bg3_plugin_lib::get_plugin_data(&path);
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

            if config.core.disabled.contains(&name.to_string()) {
                info!("Skipping plugin {name}");
                continue;
            }

            info!("Loading plugin {name}");

            let mut plugin_path = path
                .to_str()
                .ok_or(anyhow!("Failed to convert plugin path"))?
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

            // Write the data to the process
            unsafe {
                WriteProcessMemory(
                    handle.as_raw_handle(),
                    alloc_addr,
                    plugin_path.as_ptr() as *const _,
                    plugin_path_len,
                    None,
                )
                .context("Failed to write process memory")?
            }

            // start thread with dll
            unsafe {
                CreateRemoteThread(
                    handle.as_raw_handle(),
                    None,
                    0,
                    Some(LoadLibraryW),
                    Some(alloc_addr),
                    0,
                    None,
                )
                .context("Failed to create remote thread")?
            };
        }
    }

    Ok(())
}

// Determine whether the process has been tainted by previous dll injections
fn is_dirty(handle: &OwnedHandle, config: &Config) -> Result<bool> {
    let mut install_root = config.core.install_root.clone();
    // important, this must be native mods folder specifically, otherwise it will have false positives
    install_root.push("bin");
    install_root.push("NativeMods");
    let install_root = install_root.to_string_lossy().to_lowercase();

    let (_, plugins_dir) = get_bg3_plugins_dir()?;
    let plugins_dir = plugins_dir.to_string_lossy().to_lowercase();

    let is_bg3_path = move |path: String| {
        // IMPORTANT: input arg must be all lowercase!

        let dirty = path.starts_with(&install_root) || path.starts_with(&plugins_dir);

        if dirty {
            debug!("detected dirty plugin {path}");
        }

        dirty
    };

    // fully initialize this in order to set len
    let mut modules: Vec<HMODULE> = vec![HMODULE::default(); 1024];
    let mut lpcbneeded = 0;

    loop {
        let size = (modules.len() * std::mem::size_of::<HMODULE>()) as u32;

        unsafe {
            EnumProcessModulesEx(
                handle.as_raw_handle(),
                modules.as_mut_ptr(),
                size,
                &mut lpcbneeded,
                LIST_MODULES_64BIT,
            )?
        }

        // To determine if the lphModule array is too small to hold all module handles for the process,
        // compare the value returned in lpcbNeeded with the value specified in cb. If lpcbNeeded is greater
        // than cb, increase the size of the array and call EnumProcessModulesEx again.
        if lpcbneeded > size {
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
    for &module in modules {
        let len = loop {
            let len = unsafe { GetModuleFileNameExW(handle.as_raw_handle(), module, &mut name) };

            // If the buffer is too small to hold the module name, the string is truncated to nSize characters including the
            // terminating null character, the function returns nSize, and the function sets the last error to ERROR_INSUFFICIENT_BUFFER.
            if len == name.len() as u32 {
                let error = unsafe { GetLastError() };

                if error
                    .clone()
                    .is_err_and(|e| e == ERROR_INSUFFICIENT_BUFFER.into())
                {
                    name.resize(name.len() + 1024, 0u16);
                    continue;
                } else {
                    error?;
                }
            }

            // If the function fails, the return value is 0 (zero). To get extended error information, call GetLastError.
            if len == 0 {
                unsafe {
                    GetLastError()?;
                }
            }

            break len;
        };

        let path_str = String::from_utf16_lossy(&name[..len as usize]).to_lowercase();
        let path = Path::new(&path_str);

        // we're only interested in dll modules
        if path.extension().is_some_and(|ext| ext == "dll") && is_bg3_path(path_str) {
            return Ok(true);
        }
    }

    Ok(false)
}
