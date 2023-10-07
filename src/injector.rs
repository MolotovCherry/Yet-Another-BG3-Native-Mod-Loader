use std::path::Path;
use std::{ffi::c_void, fs};

use anyhow::{anyhow, bail};
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

            if config.disabled.contains(&name.to_string()) {
                info!("Skipping plugin \"{name}\"");
                continue;
            }

            info!("Loading plugin \"{name}\"");

            let mut plugin_path = path
                .to_str()
                .ok_or(anyhow!("Failed to convert plugin path"))?
                .to_owned();
            plugin_path.push('\0');

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
                bail!("Failed to allocate memory in {pid}: {err:?}");
            }

            // Write the data to the process
            unsafe {
                WriteProcessMemory(
                    handle,
                    alloc_addr,
                    plugin_path.as_bytes() as *const _ as *const _,
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
