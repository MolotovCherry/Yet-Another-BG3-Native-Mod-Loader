#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use shared::popup::fatal_popup;

fn main() {
    if let Err(e) = yabg3nml::autostart() {
        fatal_popup("autostart failure", e.to_string());
    }
}
