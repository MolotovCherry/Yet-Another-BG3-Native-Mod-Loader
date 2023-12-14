use std::fs;
use std::path::PathBuf;

use anyhow::anyhow;
use directories::BaseDirs;
use log::{debug, info};

use crate::config::Config;

pub fn get_larian_local_dir() -> anyhow::Result<PathBuf> {
    let local = BaseDirs::new().ok_or(anyhow!("Failed to instantiate BaseDirs"))?;

    let mut local = local.data_local_dir().to_owned();

    local.push("Larian Studios");

    debug!("Looking for larian local dir at: {}", local.display());

    if local.exists() {
        Ok(local)
    } else {
        Err(anyhow!("Larian local appdata directory does not exist"))
    }
}

pub fn get_bg3_local_dir() -> anyhow::Result<PathBuf> {
    let mut local = get_larian_local_dir()?;

    local.push("Baldur's Gate 3");

    debug!("Looking for bg3 local dir at: {}", local.display());

    if local.exists() {
        Ok(local)
    } else {
        Err(anyhow!("Bg3 local appdata directory does not exist"))
    }
}

pub fn get_bg3_plugins_dir() -> anyhow::Result<(bool, PathBuf)> {
    let mut plugins_dir = get_bg3_local_dir()?;
    plugins_dir.push("Plugins");

    debug!("Looking for bg3 plugins dir at: {}", plugins_dir.display());

    let mut first_time = false;

    if !plugins_dir.exists() {
        info!("Plugin directory not found; creating it..");

        fs::create_dir(&plugins_dir)?;
        first_time = true;
    }

    let log_dir = plugins_dir.join("logs");
    if !log_dir.exists() {
        fs::create_dir(plugins_dir.join("logs"))?;
    }

    Ok((first_time, plugins_dir))
}

pub fn build_config_game_binary_paths(config: &Config) -> (PathBuf, PathBuf) {
    let bin = config.core.install_root.join("bin");
    let bg3 = bin.join("bg3.exe");
    let bg3_dx11 = bin.join("bg3_dx11.exe");

    debug!("Looking for bg3 at: {}", bg3.display());
    debug!("Looking for bg3_dx11 at: {}", bg3_dx11.display());

    (bg3, bg3_dx11)
}
