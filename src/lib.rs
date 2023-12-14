mod backtrace;
mod config;
mod helpers;
mod injector;
mod panic;
mod paths;
mod popup;
mod process_watcher;
mod single_instance;
mod virtual_process_memory;

use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
};

use chrono::Local;
use human_panic::Metadata;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, TermLogger, TerminalMode, WriteLogger};

use crate::{
    injector::inject_plugins, panic::set_hook, paths::get_bg3_plugins_dir,
    process_watcher::watcher::CallType,
};

use self::{
    config::{get_config, Config},
    popup::{display_popup, fatal_popup, MessageBoxIcon},
    process_watcher::watcher::ProcessWatcher,
    single_instance::SingleInstance,
};

/// Process watcher entry point
pub fn run_watcher() {
    // TODO: singleton is broken, fix it

    // TODO: tray icon handling functionality, and exit from tray
    // This prohibits multiple app instances
    let _singleton = SingleInstance::new();

    let (plugins_dir, config) = setup();

    let bin = config.core.install_root.join("bin");
    let bg3 = config.core.install_root.join("bg3.exe");
    let bg3_dx11 = bin.join("bg3_dx11.exe");

    ProcessWatcher::watch(
        &[&bg3.to_string_lossy(), &bg3_dx11.to_string_lossy()],
        move |call| {
            if let CallType::Pid(pid) = call {
                inject_plugins(pid, &plugins_dir, &config).unwrap();
            }
        },
    )
    .unwrap()
    .wait();
}

/// Injector entry point
pub fn run_injector() {
    // This prohibits multiple app instances
    let _singleton = SingleInstance::new();

    let (plugins_dir, config) = setup();

    // 10 seconds
    let timeout = 10_000u32;

    let bin = config.core.install_root.join("bin");
    let bg3 = config.core.install_root.join("bg3.exe");
    let bg3_dx11 = bin.join("bg3_dx11.exe");

    ProcessWatcher::watch_timeout(
        &[&bg3.to_string_lossy(), &bg3_dx11.to_string_lossy()],
        timeout,
        move |call| match call {
            CallType::Pid(pid) => {
                inject_plugins(pid, &plugins_dir, &config).unwrap();
                std::process::exit(0);
            }

            CallType::Timeout => {
                fatal_popup(
                    "Fatal Error",
                    "Game process was not found. Is your `install_root` config value correct?",
                );
            }
        },
    )
    .unwrap()
    .wait();
}

fn setup() -> (PathBuf, Config) {
    // Nicely print any panic messages to the user
    set_hook(Metadata {
        name: env!("CARGO_PKG_NAME").into(),
        version: env!("CARGO_PKG_VERSION").into(),
        authors: "Cherry".into(),
        homepage: "https://github.com/MolotovCherry/Yet-Another-BG3-Native-Mod-Loader".into(),
    });

    let (first_time, plugins_dir) = match get_bg3_plugins_dir() {
        Ok(v) => v,
        Err(e) => {
            fatal_popup(
                "Fatal Error",
                format!("Failed to find bg3 plugins folder: {e}"),
            );
        }
    };

    // start logger
    setup_logs(&plugins_dir).expect("Failed to set up logs");

    // get/create config
    let config = get_config(plugins_dir.join("config.toml")).expect("Failed to get config");

    if first_time {
        display_popup(
                "Finish Setup",
                format!(
                    "The plugins folder was just created at\n{}\n\nTo install plugins, place the plugin dll files inside the plugins folder.\n\nPlease also double-check `config.toml` in the plugins folder. If you installed Steam/BG3 to a non-default path, the install root in the config needs to be adjusted before launching again.",
                    plugins_dir.display()
                ),
                MessageBoxIcon::Information,
            );
        std::process::exit(0);
    }

    (plugins_dir, config)
}

fn setup_logs<P: AsRef<Path>>(plugins_dir: P) -> anyhow::Result<()> {
    let plugins_dir = plugins_dir.as_ref();

    let date = Local::now();
    let date = date.format("%Y-%m-%d").to_string();

    let logs_dir = plugins_dir.join("logs");

    let log_path = logs_dir.join(format!("native-mod-launcher {date}.log"));

    let file = if log_path.exists() {
        match OpenOptions::new().write(true).append(true).open(log_path) {
            Ok(v) => v,
            Err(e) => {
                fatal_popup("Fatal Error", format!("Failed to open log file: {e}"));
            }
        }
    } else {
        match File::create(log_path) {
            Ok(v) => v,
            Err(e) => {
                fatal_popup("Fatal Error", format!("Failed to create log file: {e}"));
            }
        }
    };

    // enable logging
    CombinedLogger::init(vec![
        TermLogger::new(
            if cfg!(debug_assertions) {
                LevelFilter::Debug
            } else {
                LevelFilter::Info
            },
            simplelog::Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        // save log to plugins dir
        WriteLogger::new(LevelFilter::Info, simplelog::Config::default(), file),
    ])?;

    Ok(())
}
