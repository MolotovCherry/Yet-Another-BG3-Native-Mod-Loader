use std::{
    path::Path,
    sync::{Mutex, OnceLock},
};

use eyre::{Context, Result};
use native_plugin_lib::{Plugin, Version};
use tracing::{error, info};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{FreeLibrary, HMODULE},
        System::LibraryLoader::LoadLibraryW,
    },
};

static PLUGINS: OnceLock<Mutex<Vec<HMODULE>>> = OnceLock::new();

pub fn load(plugins_dir: &Path) -> Result<()> {
    _ = PLUGINS.set(Mutex::new(Vec::new()));

    let read_dir =
        std::fs::read_dir(plugins_dir).context("Failed to read plugins_dir {plugins_dir}");
    let Ok(read_dir) = read_dir else {
        error!(?read_dir, "failed to read plugins dir");

        #[cfg(not(any(debug_assertions, feature = "console")))]
        crate::popup::fatal_popup::fatal_popup(
            "Yet Another BG3 Mod Loader Error",
            format!(
                "Sttempted to read plugins dir {}, but failed opening it\n\nDo you have correct perms?\n\n{read_dir:?}",
                plugins_dir.display()
            ),
        );

        #[cfg(any(debug_assertions, feature = "console"))]
        return Ok(());
    };

    for entry in read_dir {
        let Ok(entry) = entry else {
            error!(?entry, "failed to read plugin dir file");

            #[cfg(not(any(debug_assertions, feature = "console")))]
            crate::popup::fatal_popup::fatal_popup(
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

        let mut path: Vec<u16> = dll.to_string_lossy().encode_utf16().collect();
        path.push(b'\0' as u16);
        let path = PCWSTR(path.as_ptr());

        let load_res = unsafe { LoadLibraryW(path) };
        let Ok(handle) = load_res else {
            let e = load_res.unwrap_err();
            error!(?e, "failed to load plugin {original_name}");

            #[cfg(not(any(debug_assertions, feature = "console")))]
            crate::popup::fatal_popup::fatal_popup(
                "Yet Another BG3 Mod Loader Error",
                format!("Failed to load plugin {original_name}\n\n{e}"),
            );

            #[cfg(any(debug_assertions, feature = "console"))]
            return Ok(());
        };

        let mut guard = PLUGINS.get().unwrap().lock().unwrap();
        guard.push(handle);
    }

    Ok(())
}

pub fn unload() {
    let mut guard = PLUGINS.get().unwrap().lock().unwrap();
    for plugin in guard.drain(..) {
        unsafe {
            _ = FreeLibrary(plugin);
        }
    }
}
