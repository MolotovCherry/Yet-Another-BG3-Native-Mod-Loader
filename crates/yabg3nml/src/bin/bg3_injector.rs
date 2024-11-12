#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use shared::popup::fatal_popup;
use yabg3nml::RunType;

fn main() {
    if let Err(e) = yabg3nml::run(RunType::Injector) {
        fatal_popup("injector failure", e.to_string());
    }
}
