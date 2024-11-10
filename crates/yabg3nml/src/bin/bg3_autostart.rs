#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    env,
    os::windows::process::CommandExt as _,
    path::Path,
    process::{Command, ExitCode},
};

use shared::{config::get_config, popup::fatal_popup};
use windows::Win32::System::{
    Diagnostics::Debug::DebugActiveProcessStop,
    Threading::{DEBUG_ONLY_THIS_PROCESS, DEBUG_PROCESS},
};
use yabg3nml::get_game_binary_paths;

fn main() -> ExitCode {
    // [this_exe_path, bg3_exe_path, ..args]
    let args = env::args().skip(2);

    let Some(bg3_exe) = env::args().nth(1) else {
        fatal_popup(
            "No direct launch",
            "This autostart program is not a launcher. Please check instructions for how to use it. (nth(1) missing)",
        );
    };

    let Some(bg3_exe) = Path::new(&bg3_exe).file_name().map(|p| p.to_string_lossy()) else {
        fatal_popup(
            "No direct launch",
            "This autostart program is not a launcher. Please check instructions for how to use it. (file_name() missing)",
        );
    };

    let config = match get_config() {
        Ok(v) => v,
        Err(e) => {
            fatal_popup("Error reading config", format!("Failed to get config file. Most likely either it failed to read the file, or your config file is malformed.\n\nError: {e}"));
        }
    };

    let exes = get_game_binary_paths(config);

    // validate it's actually a bg3 executable
    let is_bg3 = ["bg3.exe", "bg3_dx11.exe"].contains(&&*bg3_exe);
    if !is_bg3 {
        fatal_popup(
            "No direct launch",
            "This autostart program is not a launcher. Please check instructions for how to use it. (this is not a bg3 exe)",
        );
    }

    let bg3_path = match &*bg3_exe {
        "bg3.exe" => exes.bg3,
        "bg3_dx11.exe" => exes.bg3_dx11,
        _ => fatal_popup(
            "Unexpected error",
            format!("bg3_exe is not one of two bg3 exes. This should never happen. exe: {bg3_exe}"),
        ),
    };

    let mut child = match Command::new(bg3_path)
        .args(args)
        // bypass IFEO on this launch
        .creation_flags(DEBUG_PROCESS.0 | DEBUG_ONLY_THIS_PROCESS.0)
        .envs(env::vars())
        .spawn()
    {
        Ok(v) => v,
        Err(e) => {
            fatal_popup(
                "Spawn failure",
                format!("Failed to spawn game process: {e}"),
            );
        }
    };

    let pid = child.id();
    // stop debugging
    if let Err(e) = unsafe { DebugActiveProcessStop(pid) } {
        fatal_popup(
            "DebugActiveProcessStop failed",
            format!("DebugActiveProcessStop failed: {e}"),
        );
    }

    if let Err(e) = yabg3nml::autostart(pid) {
        fatal_popup("autostart failure", format!("{e}"));
    }

    child
        .wait()
        .map(|s| ExitCode::from(s.code().unwrap_or(1).clamp(u8::MIN as _, u8::MAX as _) as u8))
        .unwrap_or(ExitCode::FAILURE)
}
