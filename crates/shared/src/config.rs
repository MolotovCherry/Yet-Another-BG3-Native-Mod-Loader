use std::fs;
use std::path::PathBuf;

use eyre::Result;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::paths::get_bg3_plugins_dir;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub core: Core,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Core {
    /// The game's root installation directory,
    /// e.g. C:\Program Files (x86)\Steam\steamapps\common\Baldurs Gate 3
    pub install_root: PathBuf,
    /// Which plugins to disable. Each entry is the plugins filename without extension
    /// Except for those in this list, all plugins are enabled by default
    /// e.g. FooBar.dll should have an entry for "FooBar"
    pub disabled: Vec<String>,
}

impl Default for Core {
    fn default() -> Self {
        Self {
            // the default location for most people
            install_root: r"C:\Program Files (x86)\Steam\steamapps\common\Baldurs Gate 3".into(),
            disabled: Vec::new(),
        }
    }
}

pub fn get_config() -> Result<Config> {
    let path = get_bg3_plugins_dir()?.join("config.toml");

    if !path.exists() {
        let json = toml::to_string_pretty(&Config::default())?;

        if let Err(e) = fs::write(&path, json) {
            error!("failed to save config: {e}");
            return Err(e.into());
        }
    }

    let config = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(e) => {
            error!("failed to read config: {e}");
            return Err(e.into());
        }
    };

    match toml::from_str::<Config>(&config) {
        Ok(v) => Ok(v),
        Err(e) => {
            error!("failed to deserialize config: {e}");
            Err(e.into())
        }
    }
}
