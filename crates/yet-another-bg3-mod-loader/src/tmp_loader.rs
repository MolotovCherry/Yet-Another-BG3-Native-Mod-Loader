use std::{fs, path::PathBuf};

use eyre::{bail, Context, Result};

// The cdylib loader crate's data; see build.rs
static LOADER: &[u8] = include_bytes!(env!("LOADER_BIN"));
static LOADER_HASH: &str = env!("LOADER_BIN_HASH");

pub fn write_loader() -> Result<PathBuf> {
    let tmpdir = tempfile::env::temp_dir();
    if !tmpdir.exists() {
        bail!("system tmpdir does not exist; please ensure your system is set up properly");
    }

    let file = tmpdir.join(format!("loader-{LOADER_HASH}.dll"));

    if !file.exists() {
        let data = lz4_flex::decompress_size_prepended(LOADER).context("decompressing loader")?;
        fs::write(&file, data).context("writing tmp loader")?;
    }

    Ok(file)
}
