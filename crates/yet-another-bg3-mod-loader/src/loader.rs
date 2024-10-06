mod dirty;

use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::{mem, path::Path};

use eyre::{Context, Result};
use native_plugin_lib::Version;
use tracing::{error, info, trace_span};
use windows::{
    core::{s, w, Error as WinError},
    Win32::{
        Foundation::GetLastError,
        System::{
            Diagnostics::Debug::WriteProcessMemory,
            LibraryLoader::{GetModuleHandleW, GetProcAddress},
            Memory::{VirtualAllocEx, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE},
            Threading::{
                CreateRemoteThread, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION,
                PROCESS_VM_READ, PROCESS_VM_WRITE,
            },
        },
    },
};

use crate::{helpers::OwnedHandle, popup::warn_popup, process_watcher::Pid};
use dirty::is_dirty;

pub fn run_loader(pid: Pid, loader: &Path) -> Result<()> {
    let span = trace_span!("loader");
    let _guard = span.enter();

    let process: OwnedHandle = {
        let process = unsafe {
            OpenProcess(
                PROCESS_QUERY_INFORMATION
                    | PROCESS_VM_OPERATION
                    | PROCESS_VM_READ
                    | PROCESS_VM_WRITE,
                false,
                pid,
            )
        };

        match process {
            Ok(v) => v.into(),
            Err(e) => {
                error!(?e, "failed to open process");
                warn_popup("Can't open process", format!("Failed to open the game process.\n\nThis could be due to a few reasons:\n1. when the program attempted to open the process, it was already gone\n2. you need higher permissions, e.g. admin perms to open it (try running this as admin)\n\nIf this isn't a perm problem, just try again\n\nError: {e}"));
                return Ok(());
            }
        }
    };

    // get loadlibraryw address as fn pointer
    #[allow(non_snake_case)]
    let LoadLibraryW = {
        let handle = {
            let handle = unsafe { GetModuleHandleW(w!("kernel32")) };
            handle.context("Failed to get kernel32 module handle")?
        };

        let addr = unsafe { GetProcAddress(handle, s!("LoadLibraryW")) };

        let addr = addr
            .ok_or(WinError::from_win32())
            .context("failed to get LoadLibraryW proc address")?;

        type FarProc = unsafe extern "system" fn() -> isize;
        type ThreadStartRoutine = unsafe extern "system" fn(*mut c_void) -> u32;
        unsafe { mem::transmute::<FarProc, ThreadStartRoutine>(addr) }
    };

    // checks if process has already had injection done on it
    let is_dirty = match is_dirty(&process, &loader) {
        Ok(v) => v,
        Err(e) => {
            error!(?e, "failed dirty check");

            warn_popup(
                "Failed process patch check",
                format!(
                    "The process patch detection failed. Skipping process injection. Please try again.\n\n{e}",
                ),
            );

            return Ok(());
        }
    };

    if is_dirty {
        // return ok as if nothing happened, however we will log this
        warn_popup("Already patched", "The game process is already patched. If you'd like to patch it again, please restart the game and patch a fresh instance.");
        return Ok(());
    }

    let loader_formatted = {
        let data = native_plugin_lib::get_plugin_data(&loader);
        let dll_name = loader.file_name().unwrap_or_default().to_string_lossy();

        if let Ok(guard) = data {
            let data = guard.data();

            let Version {
                major,
                minor,
                patch,
            } = data.version;

            let p_name = data.name;
            let author = data.author;

            format!("{p_name} by {author} v{major}.{minor}.{patch} ({dll_name})")
        } else {
            dll_name.into_owned()
        }
    };

    info!("Running {loader_formatted}");

    let loader = loader.as_os_str().encode_wide().collect::<Vec<_>>();
    // 1 byte = u8, u16 = 2 bytes, len = number of elems in vector, so len * 2
    let loader_path_len = loader.len() * size_of::<u16>();

    let alloc_addr = {
        let addr = unsafe {
            VirtualAllocEx(
                process.as_raw_handle(),
                None,
                loader_path_len,
                MEM_RESERVE | MEM_COMMIT,
                PAGE_EXECUTE_READWRITE,
            )
        };

        if addr.is_null() {
            let error = unsafe { GetLastError() };
            error!(?error, "virtualallocex failed to allocate memory");

            warn_popup(
                "Process injection failure",
                format!("Failed to allocate memory in target process\n\nThis could be due to multiple reasons, but in any case, winapi returned an error that we cannot recover from. It's possible at this point that some plugins are injected, while others are not. Recommend restarting game and trying again\n\nError: {error:?}"),
            );

            return Ok(());
        }

        addr
    };

    // Write the data to the process
    let res = unsafe {
        WriteProcessMemory(
            process.as_raw_handle(),
            alloc_addr,
            loader.as_ptr() as *const _,
            loader_path_len,
            None,
        )
    };

    if let Err(e) = res {
        error!(?e, "Failed to write to process");
        warn_popup(
                        "Process injection failure",
                        format!("Failed to write to process memory\n\nThis could be due to multiple reasons, but in any case, winapi returned an error that we cannot recover from. It's possible at this point that some plugins are injected, while others are not. Recommend restarting game and trying again\n\nError: {e}"),
                    );
        return Ok(());
    }

    // start thread with dll
    // Note that the returned HANDLE is intentionally not closed!
    let res = unsafe {
        CreateRemoteThread(
            process.as_raw_handle(),
            None,
            0,
            Some(LoadLibraryW),
            Some(alloc_addr),
            0,
            None,
        )
    };

    if let Err(e) = res {
        error!(?e, "Failed to create remote thread");
        warn_popup(
                        "Process injection failure",
                        format!("Failed to create process remote thread\n\nThis could be due to multiple reasons, but in any case, winapi returned an error that we cannot recover from. It's possible at this point that some plugins are injected, while others are not. Recommend restarting game and trying again\n\nError: {e}"),
                    );

        return Ok(());
    }

    Ok(())
}
