#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use yet_another_bg3_mod_loader::RunType;

fn main() {
    yet_another_bg3_mod_loader::run(RunType::Watcher);
}
