use std::{
    ffi::OsString,
    os::windows::prelude::OsStringExt,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use eyre::{bail, eyre, Result};
use windows::Win32::{
    Foundation::{GetLastError, ERROR_INSUFFICIENT_BUFFER, HINSTANCE, MAX_PATH},
    System::LibraryLoader::GetModuleFileNameW,
};

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
        let error = unsafe { GetLastError() };
        if error.is_err() {
            if error == ERROR_INSUFFICIENT_BUFFER {
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
