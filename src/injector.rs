use std::{ffi::c_void, fs};
use std::{ffi::CString, path::Path};

use anyhow::{anyhow, bail};
use bg3_plugin_lib::{Plugin, Version};
use log::info;
use windows::Win32::{
    Foundation::{CloseHandle, GetLastError},
    System::{
        Diagnostics::Debug::WriteProcessMemory,
        Memory::{VirtualAllocEx, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE},
        Threading::{CreateRemoteThread, OpenProcess, PROCESS_ALL_ACCESS},
    },
};

use crate::config::Config;

windows_targets::link!("kernel32.dll" "system" fn LoadLibraryA(lplibfilename: *mut c_void) -> u32);

pub fn inject_plugins(pid: u32, plugins_dir: &Path, config: &Config) -> anyhow::Result<()> {
    let handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, false, pid)? };

    for entry in fs::read_dir(plugins_dir)? {
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

            let plugin_path = CString::new(
                path.to_str()
                    .ok_or(anyhow!("Failed to convert plugin path"))?,
            )?;
            let plugin_path = plugin_path.as_bytes_with_nul();

            let alloc_addr = unsafe {
                VirtualAllocEx(
                    handle,
                    None,
                    plugin_path.len(),
                    MEM_RESERVE | MEM_COMMIT,
                    PAGE_EXECUTE_READWRITE,
                )
            };

            // allocation failed
            if alloc_addr.is_null() {
                let err = unsafe { GetLastError() }.unwrap_err();
                bail!("Failed to allocate memory in {pid}: {err}");
            }

            // Write the data to the process
            unsafe {
                WriteProcessMemory(
                    handle,
                    alloc_addr,
                    plugin_path as *const _ as *const _,
                    plugin_path.len(),
                    None,
                )?
            }

            // start thread with dll
            unsafe {
                CreateRemoteThread(
                    handle,
                    None,
                    0,
                    Some(LoadLibraryA),
                    Some(alloc_addr),
                    0,
                    None,
                )?;
            }
        }
    }

    // Cleanup
    unsafe {
        CloseHandle(handle)?;
    }

    Ok(())
}
