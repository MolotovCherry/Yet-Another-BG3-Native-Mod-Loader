use std::{panic, path::Path};

use human_panic::metadata;
use shared::{
    backtrace::CaptureBacktrace,
    popup::{MessageBoxIcon, display_popup},
};
use tracing::error;

#[allow(unused_variables)]
pub fn set_hook() {
    let meta = metadata!();

    panic::set_hook(Box::new(move |info| {
        error!("{info}\n\nstack backtrace:\n{}", CaptureBacktrace);

        #[allow(unused_mut)]
        let mut message = info.to_string();

        // release mode, attempt to show a nice pretty popup
        #[cfg(not(debug_assertions))]
        {
            let file_path = human_panic::handle_dump(&meta, info);

            if let Ok(msg) = make_msg(file_path.as_ref()) {
                message = msg;
            }
        }

        display_popup("Oh no :(", message, MessageBoxIcon::Error);
    }));
}

#[allow(unused)]
pub fn make_msg<P: AsRef<Path>>(file_path: Option<P>) -> Result<String, std::fmt::Error> {
    use std::fmt::Write as _;

    let mut buffer = String::new();

    let name = "Yet Another BG3 Native Mod Loader";

    writeln!(buffer, "Well, this is embarrassing.\n")?;

    writeln!(
        buffer,
        "{name} had a problem and crashed. To help us diagnose the \
     problem, please send us a crash report.\n"
    )?;

    writeln!(
        buffer,
        r#"We have generated a report file at "{}".

Submit an issue with the subject of "Crash Report" and include the report as an attachment. You may also submit the relevant log file found in the plugins directory's log folder.

You can obtain extra trace information at runtime through 1 of 2 methods:

Method 1: Open a console window, set env var `YABG3NML_LOG="trace"` and run `./bg3_*.exe --cli` (replace * with the actual tool you want to run)

Method 2: go to your config.toml file and in the `[log]` section, set `level = "trace"` and run the program again

Note: It is not recommended to leave this set on `trace` for an extended period of time. This outputs a lot of debug logs, which is why the default is `info`"#,
        match file_path {
            Some(fp) => fp.as_ref().display().to_string(),
            None => "<Failed to store file to disk>".to_string(),
        }
    )?;

    writeln!(buffer, concat!("\n", env!("CARGO_PKG_HOMEPAGE")))?;

    Ok(buffer)
}
