#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

use shared::popup::{MessageBoxIcon, display_popup};

fn main() -> ExitCode {
    match yabg3nml::autostart() {
        Ok(c) => c,
        Err(e) => {
            display_popup("Autostart Failure", e.to_string(), MessageBoxIcon::Error);
            // DO NOT signal failure for the crash handler
            ExitCode::SUCCESS
        }
    }
}
