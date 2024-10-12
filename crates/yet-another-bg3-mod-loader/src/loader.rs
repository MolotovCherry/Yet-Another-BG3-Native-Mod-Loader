mod dirty;
mod write;

use std::os::windows::ffi::OsStrExt;
use std::{ffi::c_void, sync::OnceLock};
use std::{iter, sync::atomic::Ordering};
use std::{mem, path::Path};

use eyre::{Context, Result};
use native_plugin_lib::Version;
use tracing::{error, info, trace, trace_span, warn};
use windows::Win32::Foundation::WAIT_OBJECT_0;
use windows::Win32::System::Threading::{WaitForSingleObject, INFINITE, LPTHREAD_START_ROUTINE};
use windows::{
    core::{s, w, Error as WinError},
    Win32::{
        Foundation::GetLastError,
        System::{
            LibraryLoader::{GetModuleHandleW, GetProcAddress},
            Threading::{
                CreateRemoteThread, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION,
                PROCESS_VM_READ, PROCESS_VM_WRITE,
            },
        },
    },
};

use crate::{
    helpers::OwnedHandle,
    popup::warn_popup,
    process_watcher::Pid,
    server::{AUTH, PID},
    wapi::get_module_base_ex::GetModuleBaseEx,
};
use dirty::is_dirty;

use self::write::write_in;

pub fn run_loader(pid: Pid, (rva, loader): (usize, &Path)) -> Result<()> {
    let span = trace_span!("loader");
    let _guard = span.enter();

    PID.store(pid, Ordering::Release);

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
    let LoadLibraryW = 'b: {
        type FarProc = unsafe extern "system" fn() -> isize;
        type ThreadStartRoutine = unsafe extern "system" fn(*mut c_void) -> u32;

        static CACHE: OnceLock<ThreadStartRoutine> = OnceLock::new();

        if let Some(f) = CACHE.get() {
            break 'b *f;
        }

        let handle = {
            let handle = unsafe { GetModuleHandleW(w!("kernel32")) };
            handle.context("Failed to get kernel32 module handle")?
        };

        let addr = unsafe { GetProcAddress(handle, s!("LoadLibraryW")) };

        let addr = addr
            .ok_or(WinError::from_win32())
            .context("failed to get LoadLibraryW proc address")?;

        let f = unsafe { mem::transmute::<FarProc, ThreadStartRoutine>(addr) };
        _ = CACHE.set(f);
        f
    };

    // checks if process has already had injection done on it
    let is_dirty = match is_dirty(&process, loader) {
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
        warn!("Aborting patcher. The game process is already patched. If you'd like to patch it again, please restart the game and patch a fresh instance.");
        warn_popup("Already patched", "Aborting patcher. The game process is already patched. If you'd like to patch it again, please restart the game and patch a fresh instance.");
        return Ok(());
    }

    let loader_formatted = {
        let data = native_plugin_lib::get_plugin_data(loader);
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

    let loader_v = loader
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect::<Vec<_>>();
    // 1 byte = u8, u16 = 2 bytes, len = number of elems in vector, so len * 2
    let loader_path_len = loader_v.len() * size_of::<u16>();

    let Ok(ptr) = write_in(&process, loader_v.as_ptr(), loader_path_len) else {
        return Ok(());
    };

    // start thread with dll
    // Note that the returned HANDLE is intentionally not closed!
    let res = unsafe {
        CreateRemoteThread(
            process.as_raw_handle(),
            None,
            0,
            Some(LoadLibraryW),
            Some(ptr),
            0,
            None,
        )
    };

    let handle = match res {
        Ok(h) => h,
        Err(e) => {
            error!(?e, "Failed to create remote thread");
            warn_popup(
                "Process injection failure",
                format!("Failed to create process remote thread\n\nThis could be due to multiple reasons, but in any case, winapi returned an error that we cannot recover from. Recommend restarting game and trying again\n\nError: {e}"),
            );

            return Ok(());
        }
    };

    // wait for it to be done starting
    let res = unsafe { WaitForSingleObject(handle, INFINITE) };
    if res != WAIT_OBJECT_0 {
        let err = unsafe { GetLastError() };
        error!(?res, ?err, "object in wrong state");

        warn_popup(
            "Process injection failure",
            format!("Failed to wait for remote thread\n\nThis could be due to multiple reasons, but in any case, winapi returned an error that we cannot recover from. Recommend restarting game and trying again\n\nError: {err:?}"),
        );

        return Ok(());
    }

    // now call Init
    let Some(module) = GetModuleBaseEx(&process, loader) else {
        warn_popup(
            "Where is the module?",
            "Failed to find loader.dll module handle. ??? Maybe report this as a bug. Check the logs too.",
        );

        return Ok(());
    };

    let base = module.0 as usize;
    let init_addr = base + rva;

    trace!(
        base = %format!("0x{base:x}"),
        rva = %format!("0x{rva:x}"),
        addr = %format!("0x{init_addr:x}"),
        "found loader.dll Init addr"
    );

    let auth_code = rand::random::<u64>();
    AUTH.store(auth_code, Ordering::Release);

    trace!(auth_code, "generated auth");

    let Ok(ptr) = write_in(&process, &auth_code, size_of::<u64>()) else {
        return Ok(());
    };

    let init_fn = unsafe { mem::transmute::<usize, LPTHREAD_START_ROUTINE>(init_addr) };

    let res = unsafe {
        CreateRemoteThread(
            process.as_raw_handle(),
            None,
            0,
            init_fn,
            Some(ptr),
            0,
            None,
        )
    };

    if let Err(e) = res {
        error!(?e, "Failed to create remote thread");
        warn_popup(
            "Process injection failure",
            format!("Failed to create process remote thread\n\nThis could be due to multiple reasons, but in any case, winapi returned an error that we cannot recover from. Recommend restarting game and trying again\n\nError: {e}"),
        );

        return Ok(());
    }

    Ok(())
}
