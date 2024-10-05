use std::{path::PathBuf, process};

use eyre::{Context as _, Result};
use shared::{
    config::{get_config, Config},
    paths::get_bg3_plugins_dir,
};
use tracing::{error, trace};
use tracing_appender::non_blocking::WorkerGuard;

use crate::{
    cli::Args,
    logging::setup_logs,
    panic::set_hook,
    popup::{display_popup, fatal_popup, MessageBoxIcon},
    tmp_loader::write_loader,
};

pub fn init(args: &Args) -> Result<(PathBuf, Config, Option<WorkerGuard>, PathBuf)> {
    // Nicely print any panic messages to the user
    set_hook();

    let plugins_dir = match get_bg3_plugins_dir() {
        Ok(v) => v,
        Err(e) => {
            error!("failed to find plugins_dir: {e}");
            fatal_popup("Fatal Error", "Failed to find bg3 plugins folder");
        }
    };

    // start logger
    let worker_guard = setup_logs(&plugins_dir, args).context("Failed to set up logs")?;

    // get/create config
    let first_time = !plugins_dir.join("config.toml").exists();
    let config = get_config().context("Failed to get config")?;

    if first_time {
        display_popup(
                "Finish Setup",
                format!(
                    "The plugins folder was just created at\n{}\n\nTo install plugins, place the plugin dll files inside the plugins folder.\n\nPlease also double-check `config.toml` in the plugins folder. install_root in the config likely needs to be adjusted to the correct path. If the tools are placed in <bg3_root>/bin or <bg3_root>/bin/subfolder, the tools will automatically detect the correct root path and do not require install_root to be configured, otherwise you need to configure install_root",
                    plugins_dir.display()
                ),
                MessageBoxIcon::Info,
            );
        process::exit(0);
    }

    let loader = write_loader()?;

    trace!("Got config: {config:?}");

    Ok((plugins_dir, config, worker_guard, loader))
}
