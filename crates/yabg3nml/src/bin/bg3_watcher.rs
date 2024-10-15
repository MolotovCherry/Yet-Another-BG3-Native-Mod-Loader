#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eyre::Result;

use yabg3nml::RunType;

fn main() -> Result<()> {
    yabg3nml::run(RunType::Watcher)
}
