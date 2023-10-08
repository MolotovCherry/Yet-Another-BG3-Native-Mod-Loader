use std::fs;
use std::path::PathBuf;

use anyhow::anyhow;
use directories::BaseDirs;
use log::info;
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

pub fn get_larian_local_dir() -> anyhow::Result<PathBuf> {
    let local = BaseDirs::new().ok_or(anyhow!("Failed to instantiate BaseDirs"))?;

    let mut local = local.data_local_dir().to_owned();

    local.push("Larian Studios");
    if local.exists() {
        Ok(local)
    } else {
        Err(anyhow!("Larian local appdata directory does not exist"))
    }
}

pub fn get_launcher_dir() -> anyhow::Result<PathBuf> {
    let mut local = get_larian_local_dir()?;
    local.push("Launcher");
    if local.exists() {
        Ok(local)
    } else {
        Err(anyhow!("Larian launcher directory does not exist"))
    }
}

pub fn get_bg3_local_dir() -> anyhow::Result<PathBuf> {
    let mut local = get_larian_local_dir()?;

    local.push("Baldur's Gate 3");

    if local.exists() {
        Ok(local)
    } else {
        Err(anyhow!("Bg3 local appdata directory does not exist"))
    }
}

pub fn get_bg3_plugins_dir() -> anyhow::Result<(bool, PathBuf)> {
    let mut plugins_dir = get_bg3_local_dir()?;
    plugins_dir.push("Plugins");

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

pub fn get_steam_exe() -> anyhow::Result<String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let cur_ver = hkcu.open_subkey(r"Software\Valve\Steam")?;

    Ok(cur_ver.get_value("SteamExe")?)
}
