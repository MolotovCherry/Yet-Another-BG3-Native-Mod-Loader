use std::{ffi::c_void, fs};
use std::{mem, path::Path};

use anyhow::{anyhow, Context};
use bg3_plugin_lib::{Plugin, Version};
use log::info;
use windows::{
    core::{s, w, Error as WinError},
    Win32::System::{
        Diagnostics::Debug::WriteProcessMemory,
        LibraryLoader::{GetModuleHandleW, GetProcAddress},
        Memory::{VirtualAllocEx, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE},
        Threading::{
            CreateRemoteThread, OpenProcess, WaitForSingleObject, INFINITE, PROCESS_VM_OPERATION,
            PROCESS_VM_READ, PROCESS_VM_WRITE,
        },
    },
};

use crate::{config::Config, helpers::OwnedHandle, virtual_process_memory::VirtualProcessMemory};

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
            PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
            false,
            pid,
        )
        .context("Failed to OpenProcess for {pid}")?
        .into()
    };

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

            let alloc_addr = VirtualProcessMemory::new(handle.as_raw_handle(), unsafe {
                VirtualAllocEx(
                    handle.as_raw_handle(),
                    None,
                    plugin_path_len,
                    MEM_RESERVE | MEM_COMMIT,
                    PAGE_EXECUTE_READWRITE,
                )
            })
            .context("Failed to allocate virtual memory")?;

            // Write the data to the process
            unsafe {
                WriteProcessMemory(
                    handle.as_raw_handle(),
                    alloc_addr.get(),
                    plugin_path.as_ptr() as *const _,
                    plugin_path_len,
                    None,
                )
                .context("Failed to write process memory")?
            }

            // start thread with dll
            let thread: OwnedHandle = unsafe {
                CreateRemoteThread(
                    handle.as_raw_handle(),
                    None,
                    0,
                    Some(LoadLibraryW),
                    Some(alloc_addr.get()),
                    0,
                    None,
                )
                .context("Failed to create remote thread")?
                .into()
            };

            // wait for thread to finish execution
            unsafe {
                WaitForSingleObject(thread.as_raw_handle(), INFINITE);
            }
        }
    }

    Ok(())
}
