mod client;
mod helpers;
mod loader;
mod logging;
mod panic;

use std::{
    ffi::c_void,
    ptr, slice,
    sync::{LazyLock, Mutex, OnceLock},
};

use eyre::{Context as _, Error};
use helpers::{HInstance, Plugin, SuperLock};
use loader::load_plugins;
use logging::setup_logging;
use native_plugin_lib::declare_plugin;
use shared::{pipe::commands::Request, thread_data::ThreadData};
use tracing::{error, level_filters::LevelFilter, trace};
use windows::Win32::{
    Foundation::HINSTANCE,
    System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
};

use self::client::{TrySend as _, CLIENT};

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
    // Set up a custom panic hook so we can log all panics
    panic::set_hook();

    let module = *MODULE.get().unwrap();

    // extract and process thread data

    let len = {
        let mut size = [0u8; size_of::<usize>()];

        unsafe {
            ptr::copy_nonoverlapping(
                lpthreadparameter.cast::<u8>(),
                size.as_mut_ptr(),
                size_of::<usize>(),
            );
        }

        usize::from_le_bytes(size)
    };

    let ptr = unsafe { lpthreadparameter.byte_add(size_of::<usize>()) };
    let data = unsafe { slice::from_raw_parts(ptr.cast::<u8>(), len) };
    let data = serde_json::from_slice::<ThreadData>(data);

    if let Ok(ref data) = data {
        _ = CLIENT.try_send(Request::Auth(data.auth));
    }

    let level = match data {
        Ok(v) => v.level.into(),
        Err(_) => LevelFilter::OFF,
    };

    let result = panic::catch_unwind(|| {
        setup_logging(level).context("failed to setup logging")?;

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
