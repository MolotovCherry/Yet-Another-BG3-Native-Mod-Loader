use windows::{
    core::PCWSTR,
    Win32::System::Console::{
        AllocConsole, FreeConsole, GetStdHandle, SetConsoleMode, SetConsoleTitleW,
        ENABLE_PROCESSED_OUTPUT, ENABLE_QUICK_EDIT_MODE, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
        ENABLE_WRAP_AT_EOL_OUTPUT, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    },
};

/// Not meant to be run in production
#[allow(unused)]
pub fn alloc_console() -> ::windows::core::Result<()> {
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

    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE)? };

    unsafe {
        SetConsoleMode(handle, ENABLE_QUICK_EDIT_MODE)?;
    }

    let title = "Yet Another BG3 Mod Loader Debug Console"
        .encode_utf16()
        .chain(std::iter::once(0u16))
        .collect::<Vec<_>>();

    unsafe {
        SetConsoleTitleW(PCWSTR(title.as_ptr()))?;
    }

    print_intro();

    Ok(())
}

#[allow(unused)]
pub fn free_console() -> ::windows::core::Result<()> {
    unsafe {
        FreeConsole()?;
    }

    Ok(())
}

#[allow(unused)]
fn print_intro() {
    let version = env!("CARGO_PKG_VERSION");
    // short sha
    let sha = &env!("VERGEN_GIT_SHA")[..8];
    let built = env!("VERGEN_BUILD_DATE");
    let debug = cfg!(debug_assertions);

    println!(
        r#"
********************************************************************************
*                                                                              *
*                   Yet Another BG3 Mod Loader Debug Console                   *
*                                                                              *
********************************************************************************

Version {version}@{sha} (debug: {debug}) built on {built}
"#
    );
}
