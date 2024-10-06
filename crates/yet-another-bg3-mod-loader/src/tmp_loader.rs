use std::{
    fs, iter,
    os::windows::ffi::OsStrExt as _,
    path::{Path, PathBuf},
};

use eyre::{bail, Context, Result};
use windows::{
    core::{s, PCWSTR},
    Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryExW, DONT_RESOLVE_DLL_REFERENCES},
};

use crate::helpers::OwnedModule;

// The cdylib loader crate's data; see build.rs
static LOADER_BIN: &[u8] = include_bytes!(env!("LOADER_BIN"));
static LOADER_BIN_HASH: &str = env!("LOADER_BIN_HASH");

pub fn write_loader() -> Result<(usize, PathBuf)> {
    let tmpdir = tempfile::env::temp_dir();
    if !tmpdir.exists() {
        bail!("system tmpdir does not exist; please ensure your system is set up properly");
    }

    let file = tmpdir.join(format!("loader-{LOADER_BIN_HASH}.dll"));

    if !file.exists() {
        let mut out_file = fs::File::create(&file).context("decompressing loader")?;
        zstd::stream::copy_decode(LOADER_BIN, &mut out_file).context("writing tmp loader")?;
    }

    let rva = get_init_rva(&file)?;

    Ok((rva, file))
}

fn get_init_rva(loader: &Path) -> Result<usize> {
    let loader = loader
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect::<Vec<_>>();

    let loader = PCWSTR(loader.as_ptr());

    let handle: OwnedModule = unsafe {
        LoadLibraryExW(loader, None, DONT_RESOLVE_DLL_REFERENCES)
            .context("loader loadlibrary")?
            .into()
    };

    let addr = unsafe { GetProcAddress(handle.as_raw_module(), s!("Init")) };
    let Some(addr) = addr else {
        bail!("loader Init missing");
    };

    Ok(addr as usize - handle.as_raw_module().0 as usize)
}
