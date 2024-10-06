mod client;
mod helpers;
mod loader;
mod logging;
mod panic;

use std::{
    ffi::c_void,
    sync::{LazyLock, Mutex},
    thread,
};

use eyre::{Context as _, Error};
use helpers::{HInstance, Plugin, SuperLock};
use loader::load_plugins;
use logging::setup_logging;
use native_plugin_lib::declare_plugin;
use tracing::{error, trace};
use windows::Win32::{
    Foundation::HINSTANCE,
    System::{
        LibraryLoader::DisableThreadLibraryCalls,
        SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
    },
};

declare_plugin! {
    "Loader",
    "Cherry",
    "Plugin loader for Yet-Another-BG3-Mod-Loader"
}

static LOADED_PLUGINS: LazyLock<Mutex<Vec<Plugin>>> = LazyLock::new(|| Mutex::default());

#[no_mangle]
extern "C-unwind" fn DllMain(
    module: HINSTANCE,
    fdw_reason: u32,
    _lpv_reserved: *const c_void,
) -> bool {
    let module = HInstance(module);

    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            // not using these anyways
            _ = unsafe { DisableThreadLibraryCalls(module.0) };

            thread::spawn(move || {
                // Set up a custom panic hook so we can log all panics
                panic::set_hook();

                let result = panic::catch_unwind(|| {
                    setup_logging().context("failed to setup logging")?;

                    load_plugins(module)?;

                    Ok::<_, Error>(())
                });

                // If there was no panic, but error was bubbled up, then log the error
                // Panic is already logged in the hook, so we can ignore that
                if let Ok(Err(e)) = result {
                    error!("{e}");
                }
            });
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
