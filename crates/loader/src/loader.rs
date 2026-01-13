use std::{fs, iter, mem, os::windows::ffi::OsStrExt, path::PathBuf};

use eyre::{Context as _, Report, Result};
use native_plugin_lib::{Dll, PluginData, PluginError, Version};
use shared::{config::get_config, paths::get_bg3_plugins_dir, popup::warn_popup, utils::tri};
use tracing::{error, info, trace, warn};
use windows::{
    Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW},
    core::{PCWSTR, s},
};

use crate::{LOADED_PLUGINS, Plugin, utils::ThreadManager};

pub fn load_plugins() -> Result<()> {
    // SAFETY:
    // Any spawned threads MUST be joined. This is taken care of by ThreadManager,
    // but it is still an unsafe requirement that could be circumvented.
    // This function is safe because we upheld this requirement

    let plugins_dir = get_bg3_plugins_dir()?;
    let config = get_config()?.get();

    if !config.core.enabled {
        info!(
            "Plugins are globally disabled. If you want to re-enable them, set [core]enabled in config.toml to true"
        );
        return Ok(());
    }

    let read_dir = fs::read_dir(plugins_dir).context("failed to read plugins_dir {plugins_dir}");
    let Ok(read_dir) = read_dir else {
        error!(?read_dir, "failed to read plugins dir");

        warn_popup(
            "Failed to read plugins dir",
            "Attempted to read plugins dir, but failed opening it\n\nDo you have correct perms? See log for more details",
        );

        return Ok(());
    };

    let mut m = ThreadManager::new();

    for entry in read_dir {
        let Ok(entry) = entry else {
            warn!(?entry, "skipping unreadable dir entry");
            continue;
        };

        let mut path = entry.path();
        // lowercase the path for comparisons
        path.as_mut_os_str().make_ascii_lowercase();

        // not a file or dll
        if !path.is_file()
            || !path
                .extension()
                .unwrap_or_default()
                .eq_ignore_ascii_case("dll")
        {
            continue;
        }

        // check if plugin is disallowed or allowed
        let name = {
            let name = path
                .file_stem()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default();

            if name.is_empty() { "<unknown>" } else { name }
        };

        let dll = match Dll::new(&path) {
            Ok(dll) => dll,
            Err(e) => {
                error!(plugin = %name, ?e, "failed to open dll");
                continue;
            }
        };

        if dll.symbol_exists("__NOT_A_PLUGIN_DO_NOT_LOAD_OR_YOU_WILL_BE_FIRED") {
            trace!(plugin = %name, "aborting load because this is not a plugin");
            continue;
        }

        let name_formatted = {
            let data = PluginData::from_dll(dll);

            match data {
                Ok(data) => {
                    let data = data.plugin();

                    let Version {
                        major,
                        minor,
                        patch,
                    } = data.version;

                    let p_name = data.name;
                    let author = data.author;

                    format!("{p_name} by {author} v{major}.{minor}.{patch} ({name}.dll)")
                }

                Err(e) => {
                    match e {
                        // not finding it is not an error
                        PluginError::SymbolNotFound => (),
                        _ => trace!(plugin = %name, ?e),
                    }

                    format!("{name}.dll")
                }
            }
        };

        if config.core.is_plugin_disabled(name) {
            info!("Skipping disabled plugin {name_formatted}");
            continue;
        }

        info!("Loading plugin {name_formatted}");

        // do not join the handle, or it will panic
        // this is because we use ExitThread which yanks the thread out from
        // underneath rust. it does not expect this
        m.spawn({
            let name = name.to_owned();
            move || load_plugin(name, path)
        });
    }

    Ok(())
}

fn load_plugin(name: String, path: PathBuf) {
    // wrap this in try{} block and return result
    // by doing this we can return the self library guard and
    // prevent a shutdown until the end of this scope
    //
    // The purpose of doing that so we can
    let result = tri! {
        let plugin_path = path
            .as_os_str()
            .encode_wide()
            .chain(iter::once(0))
            .collect::<Vec<_>>();

        // SAFETY: Standard function, and our string is formatted properly
        let module = {
            let path = PCWSTR::from_raw(plugin_path.as_ptr());
            let res = unsafe { LoadLibraryW(path) };

            match res {
                Ok(v) => v,
                Err(e) => {
                    error!(plugin = %name, err = ?e, "failed to load library");
                    return Err(e).context("failed to load library")}
            }
        };

        // so plugin can be unloaded on dll exit
        {
            let mut plugins = LOADED_PLUGINS.lock();
            plugins.push(Plugin(module));
        }

        // SAFETY: Standard function, and again proper args
        let init = unsafe { GetProcAddress(module, s!("Init")) };
        if let Some(init) = init {
            type FarProc = unsafe extern "system" fn() -> isize;
            type Init = unsafe extern "C" fn();

            // SAFETY: We declared the signature to be `unsafe extern "C" fn()`. Implementer must abide by this
            #[allow(non_snake_case)]
            let Init = unsafe { mem::transmute::<FarProc, Init>(init) };

            trace!(plugin = %name, "running Init");

            // SAFETY: Guaranteed by implementer to not be UB
            //         Plugin is responsible
            unsafe {
                Init();
            }

            trace!(plugin = %name, "finished Init");
        }

        Ok::<_, Report>(())
    };

    if let Err(e) = result {
        error!(plugin = %name, path = %path.display(), %e, "load_plugin failed");
    }

    trace!(plugin = %name, "exit load plugin");
}
