use std::path::PathBuf;
use std::{fs, path::Path};

use directories::BaseDirs;
use eyre::{bail, eyre, Result};
use tracing::{debug, info, trace};

use crate::{config::Config, popup::fatal_popup};

pub fn get_larian_local_dir() -> Result<PathBuf> {
    let local = BaseDirs::new().ok_or(eyre!("Failed to instantiate BaseDirs"))?;

    let mut local = local.data_local_dir().to_owned();

    local.push("Larian Studios");

    trace!("Looking for larian local dir at: {}", local.display());

    if local.exists() {
        Ok(local)
    } else {
        bail!("Larian local appdata directory does not exist")
    }
}

pub fn get_bg3_local_dir() -> Result<PathBuf> {
    let mut local = get_larian_local_dir()?;

    local.push("Baldur's Gate 3");

    trace!("Looking for bg3 local dir at: {}", local.display());

    if local.exists() {
        Ok(local)
    } else {
        bail!("Bg3 local appdata directory does not exist")
    }
}

pub fn get_bg3_plugins_dir() -> Result<(bool, PathBuf)> {
    let mut plugins_dir = get_bg3_local_dir()?;
    plugins_dir.push("Plugins");

    trace!("Looking for bg3 plugins dir at: {}", plugins_dir.display());

    let mut first_time = false;

    if !plugins_dir.exists() {
        info!("Plugin directory not found; creating it..");

        fs::create_dir(&plugins_dir)?;
        first_time = true;
    }

    let log_dir = plugins_dir.join("logs");
    if !log_dir.exists() {
        info!("Log directory not found; creating it..");

        fs::create_dir(plugins_dir.join("logs"))?;
    }

    Ok((first_time, plugins_dir))
}

pub struct Bg3Exes {
    pub bg3: String,
    pub bg3_dx11: String,
}

pub fn build_config_game_binary_paths(config: &Config) -> Bg3Exes {
    let bin = config.core.install_root.join("bin");

    // first check current directory or 1 directory up for exes before using config value
    let check_dirs = [".", "..", &bin.to_string_lossy()];
    for dir in check_dirs {
        let path = Path::new(dir);

        let bg3 = path.join("bg3.exe");
        let bg3_dx11 = path.join("bg3_dx11.exe");

        if bg3.is_file() && bg3_dx11.exists() {
            let bg3 = match fs::canonicalize(&bg3) {
                Ok(p) => p,
                Err(e) => {
                    debug!(error = %e, path = %bg3.display(), "failed to canonicalize");
                    continue;
                }
            };

            let bg3_dx11 = match fs::canonicalize(&bg3_dx11) {
                Ok(p) => p,
                Err(e) => {
                    debug!(error = %e, path = %bg3_dx11.display(), "failed to canonicalize");
                    continue;
                }
            };

            // canonicalize adds this to the prefix, but we don't want it
            let bg3 = bg3
                .to_string_lossy()
                .strip_prefix(r"\\?\")
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| bg3.to_string_lossy().to_string());

            let bg3_dx11 = bg3_dx11
                .to_string_lossy()
                .strip_prefix(r"\\?\")
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| bg3_dx11.to_string_lossy().to_string());

            trace!("Looking for bg3 at: {bg3}");
            trace!("Looking for bg3_dx11 at: {bg3_dx11}");

            return Bg3Exes { bg3, bg3_dx11 };
        }
    }

    fatal_popup(
        "Path error",
        "Failed to resolve `install_root` path. Does the path (or its target) exist and point to a directory? And does this program have permissions to read that path?",
    );
}
