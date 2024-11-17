use std::{
    collections::VecDeque, env, os::windows::process::CommandExt as _, path::Path, process::Command,
};

use eyre::Result;
use shared::popup::fatal_popup;
use tracing::{error, trace};

use windows::Win32::System::{
    Diagnostics::Debug::DebugActiveProcessStop,
    Threading::{DEBUG_ONLY_THIS_PROCESS, DEBUG_PROCESS},
};

use crate::{
    event::Event, loader::run_loader, paths::get_game_binary_for, setup::init,
    single_instance::SingleInstance,
};

pub fn autostart() -> Result<()> {
    // This prohibits multiple app instances
    let _singleton = SingleInstance::new();
    let _event = Event::new()?;

    let mut init = init()?;
    let _loader_lock = init.loader.file.take();
    let _worker_guard = init.worker.take();

    // [this_exe_path, bg3_exe_path, ..args]
    let mut args = env::args().skip(1).collect::<VecDeque<_>>();

    let bg3_exe = {
        let Some(mut bg3_exe) = args.pop_front() else {
            fatal_popup(
                "No direct launch",
                "This autostart program is not a launcher. Please check instructions for how to use it. (nth(1) missing)",
            );
        };

        bg3_exe.make_ascii_lowercase();

        bg3_exe
    };

    let Some(bg3_path) = get_game_binary_for(Path::new(&bg3_exe), init.config) else {
        // it's not a bg3 executable; or at least, it's not named correctly
        fatal_popup(
            "No direct launch",
            format!("This autostart program is not a launcher. Please check instructions for how to use it. (The target - {bg3_exe} - has an incorrect filename)"),
        )
    };

    trace!(game = ?bg3_exe, ?args, "launching");
    trace!(env = ?env::vars());

    let cmd = Command::new(bg3_path)
        .args(args)
        // bypass IFEO on this launch
        .creation_flags(DEBUG_PROCESS.0 | DEBUG_ONLY_THIS_PROCESS.0)
        .envs(env::vars())
        .spawn();

    let child = match cmd {
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

    let res = run_loader(init.config, pid, &init.loader, false, true);
    if let Err(e) = res {
        error!(err = %e, "run_loader failed");
        fatal_popup(
            "run loader failed",
            format!("run_loader unexpectedly failed. You should report this.\n\nError: {e}"),
        );
    }

    Ok(())
}
