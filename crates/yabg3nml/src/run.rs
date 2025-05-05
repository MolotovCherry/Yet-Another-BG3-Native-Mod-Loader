use std::time::Duration;

use eyre::Result;
use shared::popup::{MessageBoxIcon, display_popup, fatal_popup};
use tracing::{error, trace};

#[allow(unused_imports)]
use crate::{
    event::Event,
    loader::run_loader,
    paths,
    process_watcher::{CallType, ProcessWatcher, ProcessWatcherResults, Timeout},
    setup::init,
    single_instance::SingleInstance,
    tray::AppTray,
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
    let args: crate::cli::Args = argh::from_env();

    let mut init = init()?;
    let _loader_lock = init.loader.file.take();
    let _worker_guard = init.worker.take();

    #[cfg(not(feature = "test-injection"))]
    let processes = {
        use paths::{Bg3Exes, get_game_binary_paths};
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
                let res = run_loader(init.config, pid, &init.loader, true, wait_for_init);
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
