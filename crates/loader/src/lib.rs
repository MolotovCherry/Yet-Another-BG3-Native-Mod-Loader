mod client;
mod loader;
mod logging;
mod panic_hook;
mod utils;

use std::{
    ffi::c_void,
    panic,
    sync::{LazyLock, Mutex, Once, OnceLock},
    thread,
};

use eyre::{Context as _, Error};
use native_plugin_lib::declare_plugin;
use shared::{
    pipe::commands::Request,
    popup::warn_popup,
    thread_data::ThreadData,
    utils::{OwnedHandle, SuperLock as _},
};
use tracing::{error, trace};
use windows::{
    core::w,
    Win32::{
        Foundation::HINSTANCE,
        System::{
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
            Threading::{OpenEventW, SYNCHRONIZATION_SYNCHRONIZE},
        },
    },
};

use client::{TrySend as _, CLIENT};
use loader::load_plugins;
use logging::setup_logging;
use utils::{HInstance, Plugin};

declare_plugin! {
    "Loader",
    "Cherry",
    "Plugin loader for Yet-Another-BG3-Mod-Loader"
}

static LOADED_PLUGINS: LazyLock<Mutex<Vec<Plugin>>> = LazyLock::new(Mutex::default);
static MODULE: OnceLock<HInstance> = OnceLock::new();

#[no_mangle]
extern "stdcall-unwind" fn DllMain(
    module: HINSTANCE,
    fdw_reason: u32,
    _lpv_reserved: *const c_void,
) -> bool {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            // not using these anyways
            // But disabled cause we are using crt static
            //
            // > Consider calling DisableThreadLibraryCalls when receiving DLL_PROCESS_ATTACH, unless your DLL is
            // > linked with static C run-time library (CRT).
            // _ = unsafe { DisableThreadLibraryCalls(module) };
            _ = MODULE.set(HInstance(module));

            if !is_yabg3ml() {
                unsupported_operation();
            }
        }

        DLL_PROCESS_DETACH => {
            trace!("detaching plugins");

            let mut plugins = LOADED_PLUGINS.super_lock();
            // drop all modules if we can
            plugins.clear();
        }

        _ => (),
    }

    true
}

#[no_mangle]
extern "system-unwind" fn Init(lpthreadparameter: *mut c_void) -> u32 {
    if !is_yabg3ml() {
        unsupported_operation();
        return 0;
    }

    // Set up a custom panic hook so we can log all panics
    panic_hook::set_hook();

    let module = *MODULE.get().unwrap();

    let result = panic::catch_unwind(|| {
        // extract and process thread data
        let data = unsafe { &*lpthreadparameter.cast::<ThreadData>() };
        _ = CLIENT.try_send(Request::Auth(data.auth));

        setup_logging(data.level.into()).context("failed to setup logging")?;

        load_plugins(module)?;

        Ok::<_, Error>(())
    });

    // If there was no panic, but error was bubbled up, then log the error
    // Panic is already logged in the hook, so we can ignore that
    if let Ok(Err(e)) = result {
        error!("{e}");
    }

    0
}

/// Detects if yabg3ml injected this dll.
/// This is safe to use from DllMain
fn is_yabg3ml() -> bool {
    static CACHE: OnceLock<bool> = OnceLock::new();

    *CACHE.get_or_init(|| {
        let event = unsafe {
            OpenEventW(
                SYNCHRONIZATION_SYNCHRONIZE,
                false,
                w!(r"Global\yet-another-bg3-mod-loader"),
            )
        };

        let event = event.map(OwnedHandle::new);

        event.is_ok()
    })
}

fn unsupported_operation() {
    static CALL: Once = Once::new();
    CALL.call_once(|| {
        // threaded so it won't block DllMain
        thread::spawn(|| {
            warn_popup("Unsupported Operation", "loader.dll requires YABG3ML for proper operation. No plugins have been loaded. Please use it with the support application.");
        });
    });
}
