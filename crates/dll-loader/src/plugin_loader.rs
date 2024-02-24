use std::path::Path;

use eyre::{Context, Result};
use native_plugin_lib::{Plugin, Version};
use tracing::{error, info};
use windows::{core::PCWSTR, Win32::System::LibraryLoader::LoadLibraryW};

use crate::popup::fatal_popup;

pub fn load(plugins_dir: &Path) -> Result<()> {
    let read_dir =
        std::fs::read_dir(plugins_dir).context("Failed to read plugins_dir {plugins_dir}");
    let Ok(read_dir) = read_dir else {
        error!(?read_dir, "failed to read plugins dir");
        fatal_popup(
            "Yet Another BG3 Mod Loader Error",
            format!(
                "Sttempted to read plugins dir {}, but failed opening it\n\nDo you have correct perms?\n\n{read_dir:?}",
                plugins_dir.display()
            ),
        );
    };

    for entry in read_dir {
        let Ok(entry) = entry else {
            error!(?entry, "failed to read plugin dir file");
            fatal_popup(
                "Yet Another BG3 Mod Loader Error",
                "Attempted to read file path in plugin directory, but failed\n\nDo you have correct perms?\n\n{entry:?}",
            );
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
        if let Err(e) = load_res {
            fatal_popup(
                "Yet Another BG3 Mod Loader Error",
                format!("Failed to load plugin {original_name}\n\n{e}"),
            );
        }
    }

    Ok(())
}
