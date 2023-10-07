use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    /// The game's root installation directory,
    /// e.g. C:\Program Files (x86)\Steam\steamapps\common\Baldurs Gate 3
    pub install_root: PathBuf,
    /// Extra command line flags to pass to the game upon startup
    /// --skip-launcher flag is always passed
    pub flags: Vec<String>,
    /// Use steam to launch the game, recommended to leave this enabled
    pub steam: bool,
    /// Which plugins to disable. Each entry is the plugins filename without extension
    /// Except for those in this list, all plugins are enabled by default
    /// e.g. FooBar.dll should have an entry for "FooBar"
    pub disabled: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // the default location for most people
            install_root: r"C:\Program Files (x86)\Steam\steamapps\common\Baldurs Gate 3".into(),
            flags: Vec::new(),
            steam: true,
            disabled: Vec::new(),
        }
    }
}

pub fn get_config<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
    let path = path.as_ref();

    if !path.exists() {
        let json = serde_json::to_string_pretty(&Config::default())
            .context("Failed to convert default config to json")?;

        fs::write(path, json).context("Failed to write default json to path")?;
    }

    let config = fs::read_to_string(path).context("Failed to read config to string")?;

    serde_json::from_str::<Config>(&config).context("Failed to convert json to config")
}
