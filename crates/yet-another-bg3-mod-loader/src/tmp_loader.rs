use std::{
    env,
    fs::{File, OpenOptions},
    io,
    os::windows::prelude::OpenOptionsExt,
    path::PathBuf,
};

use eyre::{Context, OptionExt as _, Result};
use pelite::{
    pe::{PeFile, Rva},
    pe64::exports::GetProcAddress,
};
use tracing::trace;
use windows::Win32::Storage::FileSystem::FILE_SHARE_READ;

use crate::popup::fatal_popup;

static LOADER_HASH: &str = env!("LOADER_HASH");

#[derive(Debug)]
pub struct Loader {
    pub rva: Rva,
    pub path: PathBuf,
    pub file: Option<File>,
}

pub fn init_loader() -> Result<Loader> {
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

    let mut file = OpenOptions::new()
        .read(true)
        // permit shared read, but no delete/rename or write until dropped
        .share_mode(FILE_SHARE_READ.0)
        .open(&loader_path)?;

    let mut data = Vec::new();
    io::copy(&mut file, &mut data)?;

    let hash = sha256::digest(&data);

    // did we compile in CI? we need default behavior if so
    let ci = option_env!("CI").is_some();
    // whether to force hash checking regardless of CI build
    let checked = option_env!("CHECK_HASH").is_some();
    let check_hash = ci || checked;

    trace!(
        expected_hash = %LOADER_HASH,
        calculated_hash = %hash,
        "This is a {} {} build",
        if ci { "CI" } else { "non-CI" },
        if check_hash {
            "checked hash"
        } else {
            "unchecked hash"
        }
    );

    if check_hash && hash != LOADER_HASH {
        fatal_popup(
            "loader dll mismatch",
            format!("loader.dll is either the wrong file, or corrupted. Please redownload the program to get a fresh copy of the exe/dll.\n\nExpected a hash of:\n{LOADER_HASH}\n\nbut instead found:\n{hash}"),
        );
    }

    let rva = get_init_rva(&data)?;

    let loader = Loader {
        rva,
        path: loader_path,
        file: Some(file),
    };

    Ok(loader)
}

fn get_init_rva(data: &[u8]) -> Result<Rva> {
    let loader = PeFile::from_bytes(&data)?;
    let rva = loader
        .get_export("Init")?
        .symbol()
        .ok_or(pelite::Error::Null)?;

    trace!(rva = %format!("0x{rva:x}"), "Found loader.dll Init rva");

    Ok(rva)
}
