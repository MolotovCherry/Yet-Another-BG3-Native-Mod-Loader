use std::path::Path;
use std::time::Duration;

use sysinfo::{PidExt, ProcessExt, ProcessRefreshKind, System, SystemExt};

use crate::{config::Config, injector::inject_plugins, popup::fatal_popup};

pub fn injector<P: AsRef<Path>>(config: Config, plugins_dir: P) -> anyhow::Result<()> {
    let plugins_dir = plugins_dir.as_ref();

    let game_bin = config.core.install_root.join("bin");
    let game_exe1 = game_bin.join("bg3.exe");
    let game_exe2 = game_bin.join("bg3_dx11.exe");

    let game_pid = get_game_pid(&[&game_exe1, &game_exe2])?;

    // now inject all the plugins into the game!
    inject_plugins(game_pid, plugins_dir, &config)?;

    Ok(())
}

fn get_game_pid(game_exes: &[&Path]) -> anyhow::Result<u32> {
    let pid;

    let mut system = System::new();
    let time = std::time::Instant::now();

    'loop_: loop {
        system.refresh_processes_specifics(ProcessRefreshKind::new());

        for proc in system.processes().values() {
            // found exact path to the process!
            if game_exes.contains(&proc.exe()) {
                pid = proc.pid().as_u32();
                break 'loop_;
            }
        }

        // stop trying if game did not launch within timeout
        if time.elapsed() >= Duration::from_secs(10) {
            // display friendlier popup
            fatal_popup(
                "Fatal Error",
                "Game process was not found. Is your `install_root` config value correct?",
            );
        }

        // give it time to open
        std::thread::sleep(Duration::from_millis(100));
    }

    Ok(pid)
}
