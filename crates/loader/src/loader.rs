use std::{fs, iter, mem, os::windows::ffi::OsStrExt, path::PathBuf, thread};

use eyre::{Context as _, Report, Result};
use native_plugin_lib::Version;
use shared::{config::get_config, pipe::commands::Command};
use tracing::{error, info, trace, warn};
use unicase::UniCase;
use windows::{
    core::{s, PCWSTR},
    Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW},
};

use crate::{
    client::{TrySend, CLIENT},
    helpers::tri,
    helpers::{FreeSelfLibrary, SuperLock},
    HInstance, Plugin, LOADED_PLUGINS,
};

pub fn load_plugins(hinstance: HInstance) -> Result<()> {
    let plugins_dir = shared::paths::get_bg3_plugins_dir()?;
    let config = get_config()?;

    let read_dir = fs::read_dir(plugins_dir).context("failed to read plugins_dir {plugins_dir}");
    let Ok(read_dir) = read_dir else {
        error!(?read_dir, "failed to read plugins dir");
        CLIENT.try_send(Command::ErrorCantReadPluginDir)?;
        return Ok(());
    };

    for entry in read_dir {
        let Ok(entry) = entry else {
            warn!(?entry, "skipping unreadable dir entry");
            continue;
        };

        let path = entry.path();
        // not a file or dll
        if !path.is_file()
            || !path
                .extension()
                .is_some_and(|e| e.to_ascii_lowercase() == "dll")
        {
            continue;
        }

        // check if plugin is disallowed or allowed
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();

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

        let is_disabled = config
            .core
            .disabled
            .iter()
            .any(|s| UniCase::new(s) == UniCase::new(name));

        if is_disabled {
            info!("Skipping disabled plugin {name_formatted}");
            continue;
        }

        info!("Loading plugin {name_formatted}");

        // do not join the handle, or it will panic
        // this is because we use ExitThread which yanks the thread out from
        // underneath rust. it does not expect this
        let name = name.to_owned();
        thread::spawn(move || load_plugin(hinstance, name, path));
    }

    Ok(())
}

fn load_plugin(hinstance: HInstance, name: String, path: PathBuf) {
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

        let mut guard = None;
        // SAFETY: Standard function, and again proper args
        let init = unsafe { GetProcAddress(main_module, s!("Init")) };
        if let Some(init) = init {
            type FarProc = unsafe extern "system" fn() -> isize;
            type Init = unsafe extern "C" fn();

            // SAFETY: We declared the signature to be `unsafe extern "C" fn()`. Implementer must abide by this
            let init = unsafe { mem::transmute::<FarProc, Init>(init) };

            // What if init runs for a long time then we try to free library in DLL_PROCESS_DETACH while it's running?
            // Here we increase self refcount to keep our dll from unloading until its time
            //
            // https://devblogs.microsoft.com/oldnewthing/20131105-00/?p=2733
            guard = Some(FreeSelfLibrary::new(hinstance.0)?);

            trace!(%name, "running Init");

            // SAFETY: Guaranteed by implementer to not be UB
            unsafe {
                init();
            }

            trace!(%name, "finished Init");
        }

        Ok::<_, Report>(guard)
    };

    if let Err(e) = result {
        error!(%name, path = %path.display(), %e, "load_plugin failed");
    }

    trace!(%name, "exit load plugin");

    // free library and exit thread here. you cannot rely on any extra code after this
}
