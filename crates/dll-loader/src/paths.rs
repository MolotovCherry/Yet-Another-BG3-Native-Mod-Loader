use std::{
    ffi::OsString,
    os::windows::prelude::OsStringExt,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use directories::BaseDirs;
use eyre::{bail, eyre, Result};
use windows::Win32::{
    Foundation::{GetLastError, ERROR_INSUFFICIENT_BUFFER, HINSTANCE, MAX_PATH},
    System::LibraryLoader::GetModuleFileNameW,
};

/// Get the larian local directory
/// `C:\Users\<user>\AppData\Local\Larian Studios`
pub fn get_larian_local_dir() -> Result<PathBuf> {
    let local = BaseDirs::new().ok_or(eyre!("Failed to instantiate BaseDirs"))?;

    let mut local = local.data_local_dir().to_owned();

    local.push("Larian Studios");
    if local.exists() {
        Ok(local)
    } else {
        bail!("Larian local appdata directory does not exist")
    }
}

/// Get the BG3 local directory
/// `C:\Users\<user>\AppData\Local\Larian Studios\Baldur's Gate 3`
pub fn get_bg3_local_dir() -> Result<PathBuf> {
    let mut local = get_larian_local_dir()?;

    local.push("Baldur's Gate 3");

    if local.exists() {
        Ok(local)
    } else {
        bail!("Bg3 local appdata directory does not exist")
    }
}

/// Get the bg3 plugins directory
/// `C:\Users\<user>\AppData\Local\Larian Studios\Baldur's Gate 3\Plugins`
pub fn get_bg3_plugins_dir() -> Result<PathBuf> {
    let mut plugins_dir = get_bg3_local_dir()?;
    plugins_dir.push("Plugins");

    if plugins_dir.exists() {
        Ok(plugins_dir)
    } else {
        bail!("BG3 Plugins dir not found")
    }
}

/// Get path to dll `<dll_dir>\myplugin.dll`
pub fn get_dll_path(module: HINSTANCE) -> Result<&'static Path> {
    static PATH: OnceLock<PathBuf> = OnceLock::new();

    if let Some(path) = PATH.get() {
        return Ok(path);
    }

    const PATH_SIZE: usize = (MAX_PATH * 2) as usize;

    // create pre-allocated stack array of correct size
    let mut path = vec![0; PATH_SIZE];

    let mut written_len;
    loop {
        // returns how many bytes written
        written_len = unsafe { GetModuleFileNameW(module, &mut path) as usize };

        // bubble up error if there was any, for example, ERROR_INSUFFICIENT_BUFFER
        let result = unsafe { GetLastError() };
        if let Err(e) = result {
            if e == ERROR_INSUFFICIENT_BUFFER.into() {
                path.resize(path.len(), 0u16);
                continue;
            }

            // unrecoverable error
            bail!("failed to call GetModuleFileNameW");
        }

        break;
    }

    let path = OsString::from_wide(&path[..written_len]);
    let path = PathBuf::from(path);
    PATH.set(path).map_err(|_| eyre!("Failed to set"))?;

    Ok(PATH.get().unwrap())
}

/// Get path to dll's parent dir
pub fn get_dll_dir(module: HINSTANCE) -> Result<PathBuf> {
    let dll_folder = get_dll_path(module)?
        .parent()
        .ok_or(eyre!("Failed to get parent of dll"))?
        .to_path_buf();

    Ok(dll_folder)
}

/// Get path to `<dll_dir>\<filename>`
pub fn get_dll_dir_filepath<P: AsRef<Path>>(module: HINSTANCE, path: P) -> Result<PathBuf> {
    Ok(get_dll_dir(module)?.join(path))
}
