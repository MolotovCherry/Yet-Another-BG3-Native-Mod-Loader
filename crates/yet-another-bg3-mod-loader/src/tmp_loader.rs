use std::{env, fs, path::PathBuf};

use eyre::{Context, OptionExt as _, Result};
use pelite::{pe::PeFile, pe64::exports::GetProcAddress};

use crate::popup::fatal_popup;

static LOADER_HASH: &str = env!("LOADER_HASH");

pub fn init_loader() -> Result<(usize, PathBuf)> {
    let current_exe_path = env::current_exe().context("unable to find current exe path")?;
    let exe_name = current_exe_path
        .file_name()
        .ok_or_eyre("filename not found")?
        .to_string_lossy();

    let loader_path = current_exe_path
        .parent()
        .ok_or_eyre("current exe parent dir not found")?
        .join("loader.dll");

    if !loader_path.exists() {
        fatal_popup(
            "Loader not found",
            format!(
                "`loader.dll` was not found. Please ensure this dll is in the same directory as {exe_name}"
            ),
        );
    }

    let data = fs::read(&loader_path)?;
    let hash = sha256::digest(&data);

    // did we compile in CI? we need default behavior if so
    let check_hash = option_env!("CI").is_some() || option_env!("CHECK_HASH").is_some();

    if check_hash && hash != LOADER_HASH {
        fatal_popup(
            "loader dll mismatch",
            format!("loader.dll is either the wrong file, or corrupted. Please redownload the program to get a fresh copy of the exe/dll.\n\nExpected a hash of:\n{LOADER_HASH}\n\nbut instead found:\n{hash}"),
        );
    }

    let rva = get_init_rva(&data)?;

    Ok((rva, loader_path))
}

fn get_init_rva(data: &[u8]) -> Result<usize> {
    let loader = PeFile::from_bytes(&data)?;
    let rva = loader
        .get_export("Init")?
        .symbol()
        .ok_or(pelite::Error::Null)?;

    Ok(rva as usize)
}
