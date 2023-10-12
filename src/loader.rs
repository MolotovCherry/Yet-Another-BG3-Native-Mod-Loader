use std::{os::windows::process::CommandExt, path::Path};
use std::{process::Command, time::Duration};

use anyhow::anyhow;
use log::info;
use sysinfo::{PidExt, ProcessExt, ProcessRefreshKind, System, SystemExt};

use crate::{
    config::Config,
    injector::inject_plugins,
    launcher_prefs::{get_launcher_preferences, save_launcher_preferences, Backend},
    paths::get_steam_exe,
    popup::{display_popup, MessageBoxIcon},
};

pub fn load<P: AsRef<Path>>(mut config: Config, plugins_dir: P) -> anyhow::Result<()> {
    let plugins_dir = plugins_dir.as_ref();

    let exe = std::env::current_exe()?;
    let exe = exe.file_name().ok_or(anyhow!("Failed to get file_name"))?;

    // true  ^ false => !true  => false
    // false ^ true  => !true  => false
    // true  ^ true  => !false => true
    // false ^ false => !false => true
    if !((exe != "bg3.exe") ^ (exe != "bg3_dx11.exe")) {
        panic!("Executable must be named \"bg3\" or \"bg3_dx11\"");
    }

    // launcher preferences must be set so it'll launch the right executable
    let backend = if exe == "bg3.exe" {
        Backend::Vulkan
    } else {
        Backend::Dx11
    };

    let mut prefs = get_launcher_preferences()?;
    info!("Using backend {backend:?}");
    prefs.default_rendering_backend = backend;
    save_launcher_preferences(&prefs)?;

    // important for launching, otherwise the launcher will show
    config.flags.push("--skip-launcher".to_owned());

    let game_bin = config.install_root.join("bin");
    let game_exe = game_bin.join(exe);
    let steam_exe = get_steam_exe()?;
    let bin_dir = if !config.steam {
        game_bin.clone()
    } else {
        Path::new(&steam_exe).parent().unwrap().to_path_buf()
    };

    let command = if config.steam {
        info!("Launching game through steam");
        config.flags = vec![format!("steam://run/1086940//{}", config.flags.join(" "))];
        steam_exe
    } else {
        // direct exe execution
        info!("Launching game from direct exe");
        info!("Game launches twice with `direct exe` method. This is not a bug. Consider using `steam` method for a seamless experience");
        game_exe.to_string_lossy().to_string()
    };

    let mut command = Command::new(command);
    let mut handle = command
        .current_dir(bin_dir)
        // CREATE_NO_WINDOW
        .creation_flags(0x08000000)
        .args(&config.flags)
        .spawn()?;

    handle.wait()?;

    let game_pid = get_game_pid(&game_exe)?;

    // now inject all the plugins into the game!
    inject_plugins(game_pid, plugins_dir, &config)?;

    Ok(())
}

fn get_game_pid(game_exe: &Path) -> anyhow::Result<u32> {
    let pid;

    let mut system = System::new();
    let time = std::time::Instant::now();

    'loop_: loop {
        system.refresh_processes_specifics(ProcessRefreshKind::new());

        for proc in system.processes().values() {
            // found exact path to the process!
            if game_exe == proc.exe() {
                pid = proc.pid().as_u32();
                break 'loop_;
            }
        }

        // stop trying if game did not launch within timeout
        if time.elapsed() >= Duration::from_secs(10) {
            // display friendlier popup
            display_popup(
                "Process not found",
                "The game process was not found. Is your `install_root` config value correct?",
                MessageBoxIcon::Error,
            );
            std::process::exit(1);
        }

        // give it time to open
        std::thread::sleep(Duration::from_millis(100));
    }

    Ok(pid)
}
