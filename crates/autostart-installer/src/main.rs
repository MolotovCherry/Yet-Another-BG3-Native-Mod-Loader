#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, io, process::ExitCode};

use shared::popup::{display_popup, fatal_popup, MessageBoxIcon};
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

const HELP: &str = r"autostart-installer

Installs bg3_autostart. Automatically patches bg3 when the game is launched, without needing to manually
run any tool. Installs registry entries at:
HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\bg3.exe
HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\bg3_dx11.exe

Flags:
    --install   install the autostarter
    --uninstall uninstall the autostarter
    --help      this message
";

fn main() -> ExitCode {
    let Some(flag) = env::args().nth(1) else {
        display_popup("Usage", HELP, MessageBoxIcon::Info);
        return ExitCode::SUCCESS;
    };

    let is_install = flag == "--install";
    let is_uninstall = flag == "--uninstall";

    if is_install {
        if let Err(e) = install() {
            fatal_popup("install failed", format!("{e}"));
        };

        display_popup("Success", "bg3_autostart was successfully installed.\n\nEvery time you launch bg3, your game will be auto patched. If you want to stop this from happening, please uninstall the tool using the provided uninstall.bat. Also, do not move bg3_autostart.exe after you install. If you wish to move it, please first uninstall, move the tool, then reinstall.\n\nPlease also note that the registry entries point at the current bg3_autostart.exe location. If this file is in your windows user folder and another windows user tries to launch the game, they may not have access to the exe in your windows user folder (since it's another windows user's files). If multiple windows users play this game, you should instead place this exe at a location accessible by all windows users to avoid this problem. Also, if you delete the tools, make sure to uninstall first!", MessageBoxIcon::Info);
        ExitCode::SUCCESS
    } else if is_uninstall {
        if let Err(e) = uninstall() {
            fatal_popup("uninstall failed", format!("If the error is \"cannot find the file specified\", you can ignore it; it simply means there was nothing to uninstall\n\n{e}"));
        };

        display_popup(
            "Success",
            "bg3_autostart was successfully uninstalled.",
            MessageBoxIcon::Info,
        );
        ExitCode::SUCCESS
    } else {
        display_popup("Usage", HELP, MessageBoxIcon::Info);
        ExitCode::SUCCESS
    }
}

const HKLM: RegKey = RegKey::predef(HKEY_LOCAL_MACHINE);
#[rustfmt::skip]
const R_BG3: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\bg3.exe";
#[rustfmt::skip]
const R_BG3_DX11: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\bg3_dx11.exe";

fn install() -> io::Result<()> {
    let (bg3_key, _) = HKLM.create_subkey(R_BG3)?;
    let (bg3_dx11_key, _) = HKLM.create_subkey(R_BG3_DX11)?;

    let cur_exe = {
        let mut c = env::current_exe()?;
        c.pop();

        c.join("bg3_autostart.exe")
    };

    if !cur_exe.exists() {
        fatal_popup(
            "Missing",
            "bg3_autostart.exe needs to be in the same folder as autostart-installer.exe",
        );
    }

    bg3_key.set_value("debugger", &&*cur_exe.to_string_lossy())?;
    bg3_dx11_key.set_value("debugger", &&*cur_exe.to_string_lossy())?;

    Ok(())
}

fn uninstall() -> io::Result<()> {
    HKLM.delete_subkey_all(R_BG3)?;
    HKLM.delete_subkey_all(R_BG3_DX11)?;

    Ok(())
}
