use std::{fs, iter, mem, os::windows::ffi::OsStrExt, path::PathBuf, thread};

use eyre::{Context as _, Report, Result};
use native_plugin_lib::Version;
use shared::{
    config::get_config,
    paths::get_bg3_plugins_dir,
    popup::warn_popup,
    utils::{tri, SuperLock as _},
};
use tracing::{error, info, trace, warn};
use windows::{
    core::{s, PCWSTR},
    Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW},
};

use crate::{Plugin, LOADED_PLUGINS};

pub fn load_plugins() -> Result<()> {
    let plugins_dir = get_bg3_plugins_dir()?;
    let config = get_config()?;

    if !config.core.enabled {
        info!("Plugins are globally disabled. If you want to re-enable them, set [core]disabled in config.toml to false or []");
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

    let mut threads = Vec::new();

    for entry in read_dir {
        let Ok(entry) = entry else {
            warn!(?entry, "skipping unreadable dir entry");
            continue;
        };

        let mut path = entry.path();
        // lowercase the path for comparisons
        path.as_mut_os_str().make_ascii_lowercase();

        // not a file or dll
        if !path.is_file() || path.extension().unwrap_or_default() != "dll" {
            continue;
        }

        // check if plugin is disallowed or allowed
        let name = {
            let name = path
                .file_stem()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default();

            if name.is_empty() {
                "<unknown>"
            } else {
                name
            }
        };

        let name_formatted = {
            let data = native_plugin_lib::get_plugin_data(&path);

            match data {
                Ok(guard) => {
                    let data = guard.data();

                    let Version {
                        major,
                        minor,
                        patch,
                    } = data.version;

                    let p_name = data.name;
                    let author = data.author;

                    format!("{p_name} by {author} v{major}.{minor}.{patch} ({name}.dll)")
                }

                Err(_) => format!("{name}.dll"),
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
        let handle = thread::spawn({
            let name = name.to_owned();
            move || load_plugin(name, path)
        });

        threads.push(handle);
    }

    // wait for all threads to be done
    for thread in threads {
        _ = thread.join();
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
        let main_module = {
            let path = PCWSTR::from_raw(plugin_path.as_ptr());
            let res = unsafe { LoadLibraryW(path) };

            match res {
                Ok(v) => v,
                Err(e) => return Err(e).context("failed to load library")
            }
        };

        // so plugin can be unloaded on dll exit
        {
            let mut plugins = LOADED_PLUGINS.super_lock();
            plugins.push(Plugin(main_module));
        }

        // SAFETY: Standard function, and again proper args
        let init = unsafe { GetProcAddress(main_module, s!("Init")) };
        if let Some(init) = init {
            type FarProc = unsafe extern "system" fn() -> isize;
            type Init = unsafe extern "C" fn();

            // SAFETY: We declared the signature to be `unsafe extern "C" fn()`. Implementer must abide by this
            let init = unsafe { mem::transmute::<FarProc, Init>(init) };

            trace!(%name, "running Init");

            // SAFETY: Guaranteed by implementer to not be UB
            unsafe {
                init();
            }

            trace!(%name, "finished Init");
        }

        Ok::<_, Report>(())
    };

    if let Err(e) = result {
        error!(%name, path = %path.display(), %e, "load_plugin failed");
    }

    trace!(%name, "exit load plugin");
}
