use std::{
    fmt::Display,
    fs,
    path::{Path, PathBuf},
};

use shared::{config::Config, popup::fatal_popup};
use tracing::{error, trace};
use unicase::UniCase;

#[allow(dead_code)]
pub struct Bg3Exes {
    pub bg3: String,
    pub bg3_dx11: String,
}

#[allow(dead_code)]
pub fn get_game_binary_paths(config: &Config) -> Bg3Exes {
    let bin = config.core.install_root.join("bin");

    // first check current directory or 1 directory up for exes before using config value
    let check_dirs = [".", "..", &bin.to_string_lossy()];
    for dir in check_dirs {
        let path = Path::new(dir);

        let bg3 = path.join("bg3.exe");
        let bg3_dx11 = path.join("bg3_dx11.exe");

        if bg3.is_file() && bg3_dx11.is_file() {
            let bg3 = match fs::canonicalize(&bg3) {
                Ok(p) => p,
                Err(e) => {
                    error!(error = %e, path = %bg3.display(), "failed to canonicalize");
                    continue;
                }
            };

            let bg3_dx11 = match fs::canonicalize(&bg3_dx11) {
                Ok(p) => p,
                Err(e) => {
                    error!(error = %e, path = %bg3_dx11.display(), "failed to canonicalize");
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

            trace!(path = %bg3, "Looking for bg3");
            trace!(path = %bg3_dx11, "Looking for bg3_dx11");

            return Bg3Exes { bg3, bg3_dx11 };
        }
    }

    fatal_popup(
        "Path error",
        "Failed to resolve `install_root` path. Does the path (or its target) exist and point to a directory? And does this program have permissions to read that path?",
    );
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum Bg3Exe {
    Vulkan,
    Dx11,
    None,
}

impl Display for Bg3Exe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let exe = match self {
            Bg3Exe::Vulkan => "vulkan",
            Bg3Exe::Dx11 => "dx11",
            Bg3Exe::None => "none",
        };

        write!(f, "{exe}")
    }
}

impl From<&Path> for Bg3Exe {
    fn from(value: &Path) -> Self {
        let Some(value) = value.file_name() else {
            return Self::None;
        };

        let value = UniCase::new(value.to_string_lossy());
        let vulkan = UniCase::new("bg3.exe");
        let dx11 = UniCase::new("bg3_dx11.exe");

        if value == vulkan {
            Self::Vulkan
        } else if value == dx11 {
            Self::Dx11
        } else {
            Self::None
        }
    }
}

#[allow(dead_code)]
pub fn get_game_binary_for(exe: impl Into<Bg3Exe>, config: &Config) -> Option<PathBuf> {
    let exe: Bg3Exe = exe.into();

    let binaries = get_game_binary_paths(config);
    let exe = match exe {
        Bg3Exe::Vulkan => binaries.bg3,
        Bg3Exe::Dx11 => binaries.bg3_dx11,
        Bg3Exe::None => return None,
    };

    Some(exe.into())
}
