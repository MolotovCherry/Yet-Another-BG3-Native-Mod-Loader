use std::{fs, path::Path};

use shared::config::Config;
use tracing::{error, trace};

use crate::popup::fatal_popup;

pub struct Bg3Exes {
    pub bg3: String,
    pub bg3_dx11: String,
}

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
