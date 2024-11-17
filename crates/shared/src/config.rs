use std::path::PathBuf;
use std::{fs, sync::LazyLock};

use eyre::{Report, Result};
use serde::{Deserialize, Serialize};
use tracing::error;
use unicase::UniCase;

use crate::paths::get_bg3_plugins_dir;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub core: Core,
    pub log: Log,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Core {
    /// The game's root installation directory,
    /// e.g. C:\Program Files (x86)\Steam\steamapps\common\Baldurs Gate 3
    pub install_root: PathBuf,
    /// Whether to globally disable or which plugins to disable.
    /// Each entry is the plugins filename without extension
    /// Except for those in this list, all plugins are enabled by default
    /// e.g. FooBar.dll should have an entry for "FooBar"
    pub disabled: Disabled,
    /// Whether to show cli window
    pub cli: bool,
}

impl Default for Core {
    fn default() -> Self {
        Self {
            // the default location for most people
            install_root: r"C:\Program Files (x86)\Steam\steamapps\common\Baldurs Gate 3".into(),
            disabled: Disabled::Global(false),
            cli: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Disabled {
    Plugins(Vec<String>),
    Global(bool),
}

impl Default for Disabled {
    fn default() -> Self {
        Self::Global(false)
    }
}

impl Disabled {
    pub fn is_global_disabled(&self) -> bool {
        matches!(self, Self::Global(true))
    }

    pub fn is_disabled(&self, name: &str) -> bool {
        match self {
            Disabled::Plugins(plugins) => {
                let name = UniCase::new(name);
                plugins.iter().any(|p| UniCase::new(p) == name)
            }

            Disabled::Global(d) => *d,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Log {
    /// configure logger level; also settlable through env var YABG3NML_LOG
    pub level: String,
    /// whether to display log targets
    pub target: bool,
}

impl Default for Log {
    fn default() -> Self {
        Self {
            level: "info".into(),
            target: Default::default(),
        }
    }
}

pub fn get_config() -> Result<&'static Config> {
    static CONFIG: LazyLock<Result<Config>> = LazyLock::new(|| {
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
    });

    CONFIG.as_ref().map_err(|e| Report::new(&**e))
}
