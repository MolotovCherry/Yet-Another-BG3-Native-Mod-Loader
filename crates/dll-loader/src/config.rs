use std::path::Path;
use std::{fs, path::PathBuf};

use eyre::Result;
use serde::{Deserialize, Serialize};

/// Need to figure out how to make a proper config?
/// quicktype.io can help you by converting json to Rust
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    // This is not part of the config, but rather used for
    // at runtime to remember where to save to
    #[serde(skip)]
    path: PathBuf,

    pub use_plugins_dir: bool,
}

impl Config {
    /// Load a config file
    /// If path doesn't exist, creates and saves default config
    /// otherwise loads what's already there
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        // if path doesn't exist, create default config,
        // save it, and return it
        if !path.exists() {
            let config = Self {
                path: path.to_owned(),
                ..Default::default()
            };

            config.save()?;
            return Ok(config);
        }

        let data = fs::read_to_string(path)?;
        let mut config = toml::from_str::<Self>(&data)?;

        // set the plugin config path
        config.path = path.to_owned();

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let serialized = toml::to_string_pretty(self)?;
        fs::write(&self.path, serialized)?;

        Ok(())
    }
}
