use std::{fs::File, path::PathBuf, process, thread};

use eyre::{Context as _, Result};
use shared::{
    config::{get_config, Config},
    paths::{get_bg3_local_dir, get_bg3_plugins_dir},
};
use tracing::{error, trace};
use tracing_appender::non_blocking::WorkerGuard;
use windows::Win32::Security::SE_DEBUG_NAME;

use crate::{
    cli::Args,
    is_admin::is_admin,
    logging::setup_logs,
    panic::set_hook,
    popup::{display_popup, fatal_popup, MessageBoxIcon},
    privileges::set_privilege,
    server::server,
    tmp_loader::init_loader,
};

#[allow(clippy::type_complexity)]
pub fn init(args: &Args) -> Result<(Config, Option<WorkerGuard>, (usize, PathBuf, File))> {
    // enable unfettered access through debug privilege if we have admin access
    if is_admin() {
        set_privilege(SE_DEBUG_NAME, true)?;
    }

    // Nicely print any panic messages to the user
    set_hook();

    let first_time = 'f: {
        let mut plugins_dir = match get_bg3_local_dir() {
            Ok(v) => v,
            Err(_) => break 'f false,
        };

        plugins_dir.push("Plugins");
        !plugins_dir.exists()
    };

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
    let config = match get_config() {
        Ok(v) => v,
        Err(e) => {
            fatal_popup("Error reading config", format!("Failed to get config file. This probably means your config file is malformed.\n\nError: {e}"));
        }
    };

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

    let loader = init_loader()?;

    trace!("Got config: {config:?}");

    thread::spawn(|| {
        let Err(e) = server();

        fatal_popup(
            "Server Error",
            format!("Pipe server unexpectedly stopped. Please report this.\n\nError:\n{e}"),
        );
    });

    Ok((config, worker_guard, loader))
}
