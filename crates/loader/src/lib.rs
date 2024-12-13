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
    core::{w, PCWSTR},
    Win32::{
        Foundation::{HINSTANCE, HMODULE},
        System::{
            LibraryLoader::{
                GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                GET_MODULE_HANDLE_EX_FLAG_PIN,
            },
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
            Threading::{OpenEventW, SYNCHRONIZATION_SYNCHRONIZE},
        },
    },
};

use client::{TrySend as _, CLIENT};
use loader::load_plugins;
use logging::setup_logging;
use utils::{Plugin, ThreadedWrapper};

declare_plugin! {
    "Loader",
    "Cherry",
    "Plugin loader for Yet-Another-BG3-Native-Mod-Loader"
}

static LOADED_PLUGINS: LazyLock<Mutex<Vec<Plugin>>> = LazyLock::new(Mutex::default);
static MODULE: OnceLock<ThreadedWrapper<HINSTANCE>> = OnceLock::new();

#[unsafe(no_mangle)]
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

            _ = MODULE.set(unsafe { ThreadedWrapper::new(module) });

            if !is_yabg3nml() {
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

/// # Safety
///
/// the param is a `*mut c_void`. The transmute is safe if T is known.
/// It is never null (since I provided a param to it), and it's not a DST.
/// It points to foreign mem provenance-wise.
///
/// We do a compile time DST check to make sure it's pointer sized
///
/// This is unsafe since the type is manually verified
#[unsafe(no_mangle)]
unsafe extern "system-unwind" fn Init(data: &ThreadData) -> u32 {
    // compile time check it's not fat
    const {
        assert!(size_of::<&ThreadData>() == size_of::<usize>());
    }

    if !is_yabg3nml() {
        unsupported_operation();
        return 0;
    }

    // ensure this library cannot be unloaded until process exit
    let module = {
        let m = unsafe { MODULE.get().unwrap_unchecked() };
        (**m).0
    };

    // Pin this dll loader in place until process exit
    _ = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_PIN | GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
            PCWSTR(module.cast()),
            // this is a discard value
            &mut HMODULE::default(),
        )
    };

    // Set up a custom panic hook so we can log all panics
    panic_hook::set_hook();

    let result = panic::catch_unwind(|| {
        // extract and process thread data
        _ = CLIENT.try_send(Request::Auth(data.auth).into());

        setup_logging(&data.log).context("failed to setup logging")?;

        // blocking call which waits for all plugins to finish DllMain/Init
        load_plugins()?;

        Ok::<_, Error>(())
    });

    // If there was no panic, but error was bubbled up, then log the error
    // Panic is already logged in the hook, so we can ignore that
    if let Ok(Err(e)) = result {
        error!("{e}");
    }

    0
}

/// Detects if yabg3nml injected this dll.
/// This is safe to use from DllMain
fn is_yabg3nml() -> bool {
    static CACHE: OnceLock<bool> = OnceLock::new();

    *CACHE.get_or_init(|| {
        let event = unsafe {
            OpenEventW(
                SYNCHRONIZATION_SYNCHRONIZE,
                false,
                w!(r"Global\yet-another-bg3-native-mod-loader"),
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
            warn_popup("Unsupported Operation", "loader.dll requires YABG3NML for proper operation. No plugins have been loaded. Please use it with the support application.");
        });
    });
}
