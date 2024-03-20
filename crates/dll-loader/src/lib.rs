mod console;
mod hook;
mod iat;
mod logging;
mod paths;
mod plugin_loader;
mod popup;
mod racy_cell;
mod utils;
mod wrapper;

use std::{ffi::c_void, sync::OnceLock};

use eyre::Result;
use log::info;
use windows::{
    core::w,
    Win32::{
        Foundation::HANDLE,
        System::{
            LibraryLoader::{DisableThreadLibraryCalls, GetModuleHandleW},
            SystemServices::DLL_PROCESS_ATTACH,
        },
    },
};
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::HINSTANCE,
        System::{
            SystemServices::DLL_PROCESS_DETACH,
            Threading::{CreateMutexA, GetCurrentProcessId},
        },
    },
};

use logging::setup_logging;

use self::wrapper::load_proxy_fns;

// Dll entry point
#[no_mangle]
extern "C-unwind" fn DllMain(
    module: HINSTANCE,
    fdw_reason: u32,
    _lpv_reserved: *const c_void,
) -> bool {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            _ = unsafe { DisableThreadLibraryCalls(module) };

            if !should_init() {
                return true;
            }

            _ = std::panic::catch_unwind(|| {
                // always spawn debug console when in debug mode, or when compiled with console feature
                #[cfg(any(debug_assertions, feature = "console"))]
                console::alloc_console().expect("Failed to alloc console");

                // set up our actual log file handling
                setup_logging(module).expect("Failed to setup logging");

                load_proxy_fns().expect("Failed to load proxy fns");

                entry().expect("entry failure");
            });
        }

        DLL_PROCESS_DETACH => {
            plugin_loader::unload();
        }

        _ => (),
    }

    true
}

fn entry() -> Result<()> {
    info!("Loading plugins from NativeMods directory");

    hook::hook()?;

    Ok(())
}

fn should_init() -> bool {
    static HANDLE: OnceLock<HANDLE> = OnceLock::new();

    let bg3 = unsafe { GetModuleHandleW(w!("bg3.exe")) };
    let bg3_dx11 = unsafe { GetModuleHandleW(w!("bg3_dx11.exe")) };

    if bg3.is_err() && bg3_dx11.is_err() {
        return false;
    }

    let name = format!("YABG3ML_DLL_LDR_{}\0", unsafe { GetCurrentProcessId() });

    let hndl = unsafe { CreateMutexA(None, true, PCSTR(name.as_ptr())) };

    match hndl {
        Ok(hndl) => {
            _ = HANDLE.set(hndl);
            true
        }
        Err(_) => false,
    }
}
