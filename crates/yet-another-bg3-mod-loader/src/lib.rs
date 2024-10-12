#![feature(never_type)]

mod cli;
mod console;
mod event;
mod helpers;
mod is_admin;
mod loader;
mod logging;
mod panic;
mod paths;
mod popup;
mod privileges;
mod process_watcher;
mod server;
mod setup;
mod single_instance;
mod tmp_loader;
mod tray;
mod wapi;

use std::time::Duration;

use clap::Parser;
use eyre::Result;
use tracing::trace;

use cli::Args;
use event::Event;
use loader::run_loader;
use popup::fatal_popup;
use process_watcher::CallType;
use process_watcher::{ProcessWatcher, Timeout};
use setup::init;
use single_instance::SingleInstance;
use tray::AppTray;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RunType {
    Watcher,
    Injector,
}

/// Process watcher entry point
pub fn run(run_type: RunType) -> Result<()> {
    // This prohibits multiple app instances
    let _singleton = SingleInstance::new();
    let _event = Event::new()?;

    let args = Args::parse();

    if args.help {
        use clap::CommandFactory;

        #[cfg(not(debug_assertions))]
        console::debug_console("Yet Another BG3 Native Mod Loader Debug Console")?;

        let mut cmd = Args::command();
        cmd.print_help()?;

        #[cfg(not(debug_assertions))]
        console::enter_to_exit()?;

        return Ok(());
    } else if args.version {
        #[cfg(not(debug_assertions))]
        console::debug_console("Yet Another BG3 Native Mod Loader Debug Console")?;

        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

        #[cfg(not(debug_assertions))]
        console::enter_to_exit()?;

        return Ok(());
    }

    let (_config, _guard, loader) = init(&args)?;
    let _loader_lock = loader.2;

    #[cfg(not(feature = "test-injection"))]
    let processes = {
        use paths::{get_game_binary_paths, Bg3Exes};
        let Bg3Exes { bg3, bg3_dx11 } = get_game_binary_paths(&_config);
        &[bg3, bg3_dx11]
    };

    #[cfg(feature = "test-injection")]
    let processes = &[args.inject];

    let (polling_rate, timeout, oneshot) = if run_type == RunType::Watcher {
        // watcher tool
        (Duration::from_secs(2), Timeout::None, false)
    } else {
        // injector tool
        (
            Duration::from_secs(1),
            Timeout::Duration(Duration::from_secs(10)),
            true,
        )
    };

    let (waiter, stop_token) =
        ProcessWatcher::new(processes, polling_rate, timeout, oneshot).run(
        move |call| match call {
                CallType::Pid(pid) => {
                    trace!("Received callback for pid {pid}, now loading");
                    run_loader(pid, (loader.0, &loader.1)).unwrap();
                }

                // only fires with injector
                CallType::Timeout => {
                    fatal_popup(
                        "Fatal Error",
                        "Game process was not found.\n\nThis can happen for 1 of 2 reasons:\n\nEither the game isn't running, so this tool timed out waiting for it\n\nOr the game wasn't detected because your `install_root` config value isn't correct\n\nIn rare cases, it could be that the program doesn't have permission to open the game process, so it skips it. In such a case, you should run this as admin (only as a last resort; in normal cases this is not needed)",
                    );
                }
            }
        );

    // tray
    if run_type == RunType::Watcher {
        AppTray::start(stop_token);
    }

    waiter.wait();

    Ok(())
}
