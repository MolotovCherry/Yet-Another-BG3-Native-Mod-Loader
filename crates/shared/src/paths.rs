use std::{fs, path::PathBuf, sync::OnceLock};

use directories::BaseDirs;
use eyre::{bail, eyre, Result};
use tracing::{info, trace};

pub fn get_larian_local_dir() -> Result<PathBuf> {
    static CACHE: OnceLock<PathBuf> = OnceLock::new();

    if let Some(cache) = CACHE.get() {
        return Ok(cache.clone());
    }

    let local = BaseDirs::new().ok_or(eyre!("Failed to instantiate BaseDirs"))?;

    let mut local = local.data_local_dir().to_owned();

    local.push("Larian Studios");

    trace!("Looking for larian local dir at: {}", local.display());

    if local.exists() {
        _ = CACHE.set(local.clone());
        Ok(local)
    } else {
        bail!("Larian local appdata directory does not exist")
    }
}

pub fn get_bg3_local_dir() -> Result<PathBuf> {
    static CACHE: OnceLock<PathBuf> = OnceLock::new();

    if let Some(cache) = CACHE.get() {
        return Ok(cache.clone());
    }

    let mut local = get_larian_local_dir()?;

    local.push("Baldur's Gate 3");

    trace!("Looking for bg3 local dir at: {}", local.display());

    if local.exists() {
        _ = CACHE.set(local.clone());
        Ok(local)
    } else {
        bail!("BG3 local appdata directory does not exist")
    }
}

pub fn get_bg3_plugins_dir() -> Result<PathBuf> {
    static CACHE: OnceLock<PathBuf> = OnceLock::new();

    if let Some(cache) = CACHE.get() {
        return Ok(cache.clone());
    }

    let mut plugins_dir = get_bg3_local_dir()?;
    plugins_dir.push("Plugins");

    trace!("Looking for bg3 plugins dir at: {}", plugins_dir.display());

    if !plugins_dir.exists() {
        info!("Plugin directory not found; creating it..");

        fs::create_dir(&plugins_dir)?;
    }

    let log_dir = plugins_dir.join("logs");
    if !log_dir.exists() {
        info!("Log directory not found; creating it..");

        fs::create_dir(plugins_dir.join("logs"))?;
    }

    _ = CACHE.set(plugins_dir.clone());
    Ok(plugins_dir)
}
