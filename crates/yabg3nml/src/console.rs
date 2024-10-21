use eyre::Result;
use windows::{
    core::PCWSTR,
    Win32::System::Console::{
        AllocConsole, GetStdHandle, SetConsoleMode, SetConsoleTitleW, ENABLE_PROCESSED_OUTPUT,
        ENABLE_VIRTUAL_TERMINAL_PROCESSING, ENABLE_WRAP_AT_EOL_OUTPUT, STD_OUTPUT_HANDLE,
    },
};

#[allow(dead_code)]
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
