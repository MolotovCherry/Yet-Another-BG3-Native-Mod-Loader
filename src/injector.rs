use std::fs;
use std::{mem, path::Path};

use anyhow::{anyhow, bail};
use bg3_plugin_lib::{Plugin, Version};
use log::info;
use windows::Win32::{
    Foundation::{CloseHandle, GetLastError},
    System::{
        Diagnostics::Debug::WriteProcessMemory,
        Memory::{VirtualAllocEx, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE},
        Threading::{
            CreateRemoteThread, OpenProcess, PROCESS_VM_OPERATION, PROCESS_VM_READ,
            PROCESS_VM_WRITE,
        },
    },
};
use windows_sys::Win32::System::LibraryLoader::LoadLibraryW;

use crate::config::Config;

#[allow(non_snake_case)]
pub fn inject_plugins(pid: u32, plugins_dir: &Path, config: &Config) -> anyhow::Result<()> {
    // cast from fn item to fn pointer
    let LoadLibraryW = LoadLibraryW as unsafe extern "system" fn(_) -> _;

    let handle = unsafe {
        OpenProcess(
            PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
            false,
            pid,
        )?
    };

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
                    handle,
                    None,
                    plugin_path_len,
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
                    plugin_path.as_ptr() as *const _,
                    plugin_path_len,
                    None,
                )?
            }

            // start thread with dll
            unsafe {
                CreateRemoteThread(
                    handle,
                    None,
                    0,
                    Some(mem::transmute(LoadLibraryW)),
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
