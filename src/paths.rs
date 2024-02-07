use std::fs;
use std::path::PathBuf;

use anyhow::anyhow;
use directories::BaseDirs;
use log::{debug, info};

use crate::{config::Config, popup::fatal_popup};

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

pub fn build_config_game_binary_paths(config: &Config) -> (String, String) {
    let canon = fs::canonicalize(&config.core.install_root);
    let Ok(resolved_path) = canon else {
        fatal_popup(
            "Path error",
            format!("Failed to resolve `install_root` path. Does the path (or its target) exist and point to a directory? And does this program have permissions to read that path?\n\n{canon:#?}"),
        );
    };

    let bin = resolved_path.join("bin");

    let bg3 = bin.join("bg3.exe");
    let bg3_dx11 = bin.join("bg3_dx11.exe");

    let bg3 = bg3.to_string_lossy();
    let bg3_dx11 = bg3_dx11.to_string_lossy();

    // canonicalize adds this to the prefix, but we don't want it
    let bg3 = bg3.strip_prefix(r"\\?\").unwrap_or(&*bg3).to_string();
    let bg3_dx11 = bg3_dx11.strip_prefix(r"\\?\").unwrap_or(&*bg3).to_string();

    debug!("Looking for bg3 at: {bg3}");
    debug!("Looking for bg3_dx11 at: {bg3_dx11}");

    (bg3, bg3_dx11)
}
