use std::{
    collections::VecDeque,
    env,
    os::windows::process::{CommandExt as _, ExitCodeExt as _},
    path::Path,
    process::{Command, ExitCode},
    thread,
};

use eyre::{eyre, Result};
use shared::popup::fatal_popup;
use tracing::{error, trace};

use windows::Win32::System::{
    Diagnostics::Debug::DebugActiveProcessStop,
    Threading::{DEBUG_ONLY_THIS_PROCESS, DEBUG_PROCESS},
};

use crate::{
    event::Event,
    loader::run_loader,
    paths::{get_game_binary_for, Bg3Exe},
    setup::init,
    single_instance::SingleInstance,
    wapi::event_loop::EventLoop,
};

pub fn autostart() -> Result<ExitCode> {
    // This prohibits multiple app instances
    let _singleton = SingleInstance::new();
    let _event = Event::new()?;

    let mut init = init()?;
    let _loader_lock = init.loader.file.take();
    let _worker_guard = init.worker.take();

    // [this_exe_path, bg3_exe_path, ..args]
    let mut args = env::args().skip(1).collect::<VecDeque<_>>();

    let bg3_exe = {
        let Some(bg3_exe) = args.pop_front() else {
            fatal_popup(
                "No direct launch",
                "This autostart program is not a launcher. Please check instructions for how to use it. (nth(1) missing)",
            );
        };

        bg3_exe
    };

    let exe: Bg3Exe = Path::new(&bg3_exe).into();
    let Some(bg3_path) = get_game_binary_for(exe, init.config) else {
        // it's not a bg3 executable; or at least, it's not named correctly
        fatal_popup(
            "No direct launch",
            format!("This autostart program is not a launcher. Please check instructions for how to use it. (The target - {bg3_exe} - has an incorrect filename)"),
        )
    };

    trace!(mode = %exe, ?args, "launching bg3");
    trace!(env = ?env::vars());

    let cmd = Command::new(bg3_path)
        .args(args)
        // bypass IFEO on this launch
        .creation_flags(DEBUG_PROCESS.0 | DEBUG_ONLY_THIS_PROCESS.0)
        .envs(env::vars())
        .spawn();

    let mut child = match cmd {
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

    // just put something here to stop the needless busy cursor
    thread::spawn(|| EventLoop::new().run(|_, _| ()));

    match child.wait() {
        Ok(status) => {
            trace!(code = status.code(), "original child exit code");

            let code = status
                .code()
                .map(|c| ExitCode::from_raw(c as u32))
                .unwrap_or(ExitCode::FAILURE);

            Ok(code)
        }

        Err(error) => {
            error!(%error, "failed to wait for child");
            Err(eyre!("{error}"))
        }
    }
}
