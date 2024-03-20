use std::{
    path::Path,
    sync::{Mutex, OnceLock},
};

use eyre::{Context, Result};
use log::{error, info};
use native_plugin_lib::{Plugin, Version};
use windows::{
    core::{s, PCWSTR},
    Win32::{
        Foundation::{FreeLibrary, HMODULE},
        System::LibraryLoader::{GetProcAddress, LoadLibraryW},
    },
};

use crate::popup::fatal_popup;

static PLUGINS: OnceLock<Mutex<Vec<HMODULE>>> = OnceLock::new();

pub fn load(plugins_dir: &Path) -> Result<()> {
    _ = PLUGINS.set(Mutex::new(Vec::new()));

    let read_dir =
        std::fs::read_dir(plugins_dir).context("Failed to read plugins_dir {plugins_dir}");
    let Ok(read_dir) = read_dir else {
        error!("failed to read plugins dir: {:?}", read_dir);

        #[cfg(not(any(debug_assertions, feature = "console")))]
        fatal_popup(
            "Yet Another BG3 Mod Loader Error",
            format!(
                "Sttempted to read plugins dir {}, but failed opening it\n\nDo you have correct perms?\n\n{read_dir:?}",
                plugins_dir.display()
            ),
        );

        #[cfg(any(debug_assertions, feature = "console"))]
        return Ok(());
    };

    let mut guard = PLUGINS.get().unwrap().lock().unwrap();

    for entry in read_dir {
        let Ok(entry) = entry else {
            error!("failed to read plugin dir file: {:?}", entry);

            #[cfg(not(any(debug_assertions, feature = "console")))]
            fatal_popup(
                "Yet Another BG3 Mod Loader Error",
                "Attempted to read file path in plugin directory, but failed\n\nDo you have correct perms?\n\n{entry:?}",
            );

            #[cfg(any(debug_assertions, feature = "console"))]
            return Ok(());
        };

        let dll = entry.path();
        let is_dll = dll.is_file()
            && dll
                .extension()
                .is_some_and(|ext| ext.to_ascii_lowercase() == "dll");

        if !is_dll {
            continue;
        }

        let name = dll.file_stem().unwrap_or_default().to_string_lossy();
        let original_name = format!("{name}.dll");

        let info = native_plugin_lib::get_plugin_data(dll.clone());
        let name = if let Ok(data) = info {
            let Plugin {
                version:
                    Version {
                        major,
                        minor,
                        patch,
                    },
                ..
            } = data;

            data.get_name()
                .map(|n| format!("{n} v{major}.{minor}.{patch} ({name}.dll)"))
                .unwrap_or(format!("{name}.dll v{major}.{minor}.{patch}"))
        } else {
            original_name.clone()
        };

        info!("Loading plugin {name}");

        let path: Vec<u16> = dll
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let load_res = unsafe { LoadLibraryW(PCWSTR(path.as_ptr())) };
        let Ok(handle) = load_res else {
            let e = load_res.unwrap_err();
            error!("failed to load plugin {original_name}: {e:?}");

            #[cfg(not(any(debug_assertions, feature = "console")))]
            fatal_popup(
                "Yet Another BG3 Mod Loader Error",
                format!("Failed to load plugin {original_name}\n\n{e}"),
            );

            #[cfg(any(debug_assertions, feature = "console"))]
            return Ok(());
        };

        let init = unsafe { GetProcAddress(handle, s!("Init")) };
        if let Some(init) = init {
            let result = unsafe { init() };
            if result != 2 {
                fatal_popup(
                    "Yet Another BG3 Mod Loader Error",
                    format!("Plugin {original_name} Init() crashed. There's a problem with the plugin. Please contact the plugin author"),
                );
            }
        }

        guard.push(handle);
    }

    Ok(())
}

pub fn unload() {
    let Some(Ok(mut guard)) = PLUGINS.get().map(|g| g.lock()) else {
        return;
    };

    for plugin in guard.drain(..) {
        unsafe {
            _ = FreeLibrary(plugin);
        }
    }
}
