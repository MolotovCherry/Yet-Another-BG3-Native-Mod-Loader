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
mod server;
mod setup;
mod single_instance;
mod stop_token;
mod tmp_loader;
mod tray;
mod wapi;

use std::{
    env,
    os::windows::process::CommandExt as _,
    path::Path,
    process::{Command, ExitCode},
    time::Duration,
};

use eyre::Result;
use shared::popup::{display_popup, fatal_popup, MessageBoxIcon};
use tracing::{error, trace};

#[cfg(feature = "test-injection")]
use cli::Args;
use event::Event;
use loader::run_loader;
use process_watcher::{CallType, ProcessWatcher, ProcessWatcherResults, Timeout};
use setup::init;
use single_instance::SingleInstance;
use tray::AppTray;

#[allow(unused)]
pub use paths::get_game_binary_paths;
use windows::Win32::System::{
    Diagnostics::Debug::DebugActiveProcessStop,
    Threading::{DEBUG_ONLY_THIS_PROCESS, DEBUG_PROCESS},
};

#[derive(Copy, Clone, Debug)]
pub enum RunType {
    Watcher,
    Injector,
}

/// Process watcher entry point
pub fn run(run_type: RunType) -> Result<()> {
    // This prohibits multiple app instances
    let _singleton = SingleInstance::new();
    let _event = Event::new()?;

    #[cfg(feature = "test-injection")]
    let args: Args = argh::from_env();

    let mut init = init()?;
    let _loader_lock = init.loader.file.take();
    let _worker_guard = init.worker.take();

    #[cfg(not(feature = "test-injection"))]
    let processes = {
        use paths::{get_game_binary_paths, Bg3Exes};
        let Bg3Exes { bg3, bg3_dx11 } = get_game_binary_paths(init.config);
        &[bg3, bg3_dx11]
    };

    #[cfg(feature = "test-injection")]
    let processes = &[args.inject];

    let (polling_rate, timeout, oneshot, wait_for_init) = if matches!(run_type, RunType::Watcher) {
        // watcher tool
        (Duration::from_secs(2), Timeout::None, false, false)
    } else {
        // injector tool
        (
            Duration::from_secs(1),
            Timeout::Duration(Duration::from_secs(10)),
            true,
            true,
        )
    };

    let ProcessWatcherResults {
        watcher_token: token,
        watcher_handle,
        timeout_token,
    } = ProcessWatcher::new(processes, polling_rate, timeout, oneshot).run(
        move |call| match call {
            CallType::Pid(pid) => {
                trace!(pid, "Received callback for pid, now loading");
                let res = run_loader(init.config, pid, &init.loader, wait_for_init);
                if let Err(e) = res {
                    error!(err = %e, "run_loader failed");
                    fatal_popup(
                        "run loader failed",
                        format!(
                            "run_loader unexpectedly failed. You should report this.\n\nError: {e}"
                        ),
                    );
                }
            }

            // only fires with injector
            CallType::Timeout => {
                display_popup(
                    "Timed Out",
                    r"Game process was not found.

This can happen for 1 of 3 reasons:

1. The game isn't running, so this tool timed out waiting for it

2. The game wasn't detected because your `install_root` config value isn't correct

3. In rare cases, it could be that the program doesn't have permission to open the game process, so it never sees it. In such a case, you should run this as admin (only as a last resort; in normal cases this is not needed)",
                    MessageBoxIcon::Error,
                );
            }
        },
    );

    let tray = AppTray::run(token, timeout_token, run_type);
    if matches!(run_type, RunType::Watcher) {
        // will exit when Quit clicked
        _ = tray.join();
    }

    // will exit when signal sent
    _ = watcher_handle.join();

    Ok(())
}

pub fn autostart() -> Result<ExitCode> {
    // This prohibits multiple app instances
    let _singleton = SingleInstance::new();
    let _event = Event::new()?;

    let mut init = init()?;
    let _loader_lock = init.loader.file.take();
    let _worker_guard = init.worker.take();

    // [this_exe_path, bg3_exe_path, ..args]
    let args = env::args().skip(2);

    let Some(bg3_exe) = env::args().nth(1) else {
        fatal_popup(
            "No direct launch",
            "This autostart program is not a launcher. Please check instructions for how to use it. (nth(1) missing)",
        );
    };

    let Some(bg3_exe) = Path::new(&bg3_exe)
        .file_name()
        .map(|p| p.to_string_lossy().to_lowercase())
    else {
        fatal_popup(
            "No direct launch",
            "This autostart program is not a launcher. Please check instructions for how to use it. (file_name() missing)",
        );
    };

    let exes = get_game_binary_paths(init.config);

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
        _ => unreachable!(),
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

    let res = run_loader(init.config, pid, &init.loader, true);
    if let Err(e) = res {
        error!(err = %e, "run_loader failed");
        fatal_popup(
            "run loader failed",
            format!("run_loader unexpectedly failed. You should report this.\n\nError: {e}"),
        );
    }

    let code = child
        .wait()
        .map(|s| ExitCode::from(s.code().unwrap_or(1).clamp(u8::MIN as _, u8::MAX as _) as u8))
        .unwrap_or(ExitCode::FAILURE);

    Ok(code)
}
