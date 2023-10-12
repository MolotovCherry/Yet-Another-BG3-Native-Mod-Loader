mod backtrace;
mod config;
mod injector;
mod launcher_prefs;
mod loader;
mod panic;
mod paths;
mod popup;

use std::{
    fs::{File, OpenOptions},
    path::Path,
};

use chrono::Local;
use human_panic::Metadata;
use simplelog::*;

use crate::{panic::set_hook, paths::get_bg3_plugins_dir};

use self::{
    config::get_config,
    loader::load,
    popup::{display_popup, MessageBoxIcon},
};

pub fn run() {
    // Nicely print any panic messages to the user
    set_hook(Metadata {
        name: env!("CARGO_PKG_NAME").into(),
        version: env!("CARGO_PKG_VERSION").into(),
        authors: "Cherry".into(),
        homepage: "https://github.com/MolotovCherry/Yet-Another-BG3-Native-Mod-Loader".into(),
    });

    let (first_time, plugins_dir) =
        get_bg3_plugins_dir().expect("Failed to get Bg3 plugin directory");

    // start logger
    setup_logs(&plugins_dir).expect("Failed to set up logs");

    // get/create config
    let config = get_config(plugins_dir.join("config.json")).expect("Failed to get config");

    if first_time {
        display_popup(
            "Finish Setup",
            &format!(
                "The plugins folder was just created at\n{}\n\nTo install plugins, place the plugin dll files inside the plugins folder.\n\nPlease also double-check `config.json` in the plugins folder. If you installed Steam/BG3 to a non-default path, the install root in the config needs to be adjusted before launching again.",
                plugins_dir.display()
            ),
            MessageBoxIcon::Information,
        );
        std::process::exit(0);
    }

    load(config, plugins_dir).expect("Failed to load");
}

fn setup_logs<P: AsRef<Path>>(plugins_dir: P) -> anyhow::Result<()> {
    let plugins_dir = plugins_dir.as_ref();

    let date = Local::now();
    let date = date.format("%Y-%m-%d").to_string();

    let logs_dir = plugins_dir.join("logs");

    let log_path = logs_dir.join(format!("native-mod-launcher {date}.log"));

    let file = if log_path.exists() {
        OpenOptions::new().write(true).append(true).open(log_path)?
    } else {
        File::create(log_path)?
    };

    // enable logging
    CombinedLogger::init(vec![
        TermLogger::new(
            if cfg!(debug_assertions) {
                LevelFilter::Debug
            } else {
                LevelFilter::Info
            },
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        // save log to plugins dir
        WriteLogger::new(LevelFilter::Info, Config::default(), file),
    ])?;

    Ok(())
}
