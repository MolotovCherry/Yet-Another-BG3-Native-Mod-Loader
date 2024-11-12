#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

use shared::popup::fatal_popup;

fn main() -> ExitCode {
    match yabg3nml::autostart() {
        Ok(c) => c,
        Err(e) => fatal_popup("injector failure", e.to_string()),
    }
}
