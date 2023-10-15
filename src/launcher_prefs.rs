use std::{collections::BTreeMap, fs};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{paths::get_launcher_dir, popup::fatal_popup};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LauncherPreferences {
    // Use default value if field is missing
    // dx11 is default if field is missing, so we can skip serializing it
    #[serde(default)]
    #[serde(skip_serializing_if = "is_dx11")]
    pub default_rendering_backend: Backend,
    // Use BTreeMap in order to keep the file order the same
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Default, PartialEq, Debug)]
pub enum Backend {
    // 2 also is valid for vulkan
    // However, only 0 selects in the interface for some reason, so 0 should be used for serializing vulkan
    Vulkan = 0,
    // If field is missing, Dx11 is the default
    #[default]
    Dx11 = 1,
}

fn is_dx11(val: &Backend) -> bool {
    *val == Backend::Dx11
}

impl serde::Serialize for Backend {
    #[allow(clippy::use_self)]
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let value: u8 = match *self {
            Backend::Vulkan => Backend::Vulkan as u8,
            Backend::Dx11 => Backend::Dx11 as u8,
        };
        serde::Serialize::serialize(&value, serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Backend {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match <u8 as serde::Deserialize>::deserialize(deserializer)? {
            // Important! These are both valid for Vulkan
            0 | 2 => Ok(Backend::Vulkan),
            1 => Ok(Backend::Dx11),
            // For all other values, Vulkan is the one actually launched
            _ => Ok(Backend::Vulkan),
        }
    }
}

pub fn get_launcher_preferences() -> anyhow::Result<LauncherPreferences> {
    let mut preferences = get_launcher_dir()?;
    preferences.push("Settings");
    preferences.push("preferences.json");

    if !preferences.exists() {
        fatal_popup("Fatal Error", "Launcher preferences.json not found");
    }

    let data = match std::fs::read_to_string(&preferences) {
        Ok(v) => v,
        Err(e) => {
            fatal_popup(
                "Fatal Error",
                format!("Could not read launcher preferences.json: {e}"),
            );
        }
    };

    let config = match serde_json::from_str::<LauncherPreferences>(&data) {
        Ok(v) => v,
        Err(e) => {
            fatal_popup(
                "Fatal Error",
                format!("Failed to deserialize launcher preferences.json: {e}\n\nYour config probably has a mistake in it. Please verify it's correctly formatted as json"),
            );
        }
    };

    Ok(config)
}

pub fn save_launcher_preferences(prefs: &LauncherPreferences) -> anyhow::Result<()> {
    let mut path = get_launcher_dir()?;
    path.push("Settings");
    path.push("preferences.json");

    let data = serde_json::to_string_pretty(&prefs)?;

    if let Err(e) = fs::write(path, data) {
        fatal_popup(
            "Fatal Error",
            format!("Failed to save launcher preferences.json: {e}"),
        );
    }

    Ok(())
}
