mod config;
mod console;
mod logging;
mod paths;
mod plugin_loader;
mod popup;
mod wrapper;

use std::ffi::c_void;

use eyre::Result;
use tracing::info;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::Win32::{Foundation::HINSTANCE, System::SystemServices::DLL_PROCESS_DETACH};

use config::Config;
use logging::setup_logging;
use paths::get_dll_dir_filepath;

use self::paths::get_bg3_plugins_dir;

// Dll entry point
#[no_mangle]
extern "C-unwind" fn DllMain(
    module: HINSTANCE,
    fdw_reason: u32,
    _lpv_reserved: *const c_void,
) -> bool {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            _ = std::panic::catch_unwind(|| {
                // always spawn debug console when in debug mode, or when compiled with console feature
                #[cfg(any(debug_assertions, feature = "console"))]
                console::alloc_console().expect("Failed to alloc console");

                // set up our actual log file handling
                setup_logging(module).expect("Failed to setup logging");

                entry(module).expect("entry failure");
            });
        }

        DLL_PROCESS_DETACH => {
            plugin_loader::unload();
        }

        _ => (),
    }

    true
}

fn entry(module: HINSTANCE) -> Result<()> {
    let config_path = get_dll_dir_filepath(module, "yet-another-bg3-mod-loader.toml")?;
    let config = Config::load(config_path).unwrap_or_default();

    let plugins_dir = if config.use_plugins_dir {
        info!("Loading plugins from NativeMods directory");
        get_dll_dir_filepath(module, "NativeMods")?
    } else {
        info!("Loading plugins from local Plugins directory");
        get_bg3_plugins_dir()?
    };

    plugin_loader::load(&plugins_dir)?;

    Ok(())
}
