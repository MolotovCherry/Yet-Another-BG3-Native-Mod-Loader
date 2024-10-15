use std::io::Write as _;

use eyre::Result;
use windows::{
    core::PCWSTR,
    Win32::System::Console::{
        AllocConsole, GetStdHandle, SetConsoleMode, SetConsoleTitleW, ENABLE_PROCESSED_OUTPUT,
        ENABLE_VIRTUAL_TERMINAL_PROCESSING, ENABLE_WRAP_AT_EOL_OUTPUT, STD_OUTPUT_HANDLE,
    },
};

#[allow(unused)]
pub fn debug_console<A: AsRef<str>>(title: A) -> Result<()> {
    unsafe {
        AllocConsole()?;
    }

    let handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE)? };

    unsafe {
        SetConsoleMode(
            handle,
            ENABLE_PROCESSED_OUTPUT
                | ENABLE_WRAP_AT_EOL_OUTPUT
                | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
        )?;
    }

    let title = title
        .as_ref()
        .encode_utf16()
        .chain(std::iter::once(0u16))
        .collect::<Vec<_>>();

    unsafe {
        SetConsoleTitleW(PCWSTR(title.as_ptr()))?;
    }

    Ok(())
}

#[allow(unused)]
pub fn enter_to_exit() -> Result<()> {
    print!("\nPress ENTER to exit..");
    std::io::stdout().flush()?;

    // empty std input
    std::io::stdin().read_line(&mut String::new())?;

    Ok(())
}
