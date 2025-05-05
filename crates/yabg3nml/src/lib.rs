#![feature(windows_process_exit_code_from)]

mod autostart;
mod cli;
mod console;
mod event;
mod is_admin;
mod loader;
mod logging;
mod panic;
mod paths;
mod privileges;
mod process_watcher;
mod remote_thread;
mod run;
mod server;
mod setup;
mod single_instance;
mod stop_token;
mod tmp_loader;
mod tray;
mod utils;
mod wapi;

pub use autostart::autostart;
pub use run::{RunType, run};
