use std::{panic, path::Path};

use human_panic::Metadata;
use tracing::error;

use crate::{
    backtrace::CaptureBacktrace,
    popup::{display_popup, MessageBoxIcon},
};

#[allow(unused_variables)]
pub fn set_hook(meta: Metadata) {
    panic::set_hook(Box::new(move |info| {
        error!("{info}\n\nstack backtrace:\n{}", CaptureBacktrace);

        #[allow(unused_mut)]
        let mut message = info.to_string();

        // release mode, attempt to show a nice pretty popup
        #[cfg(not(debug_assertions))]
        {
            let file_path = human_panic::handle_dump(&meta, info);

            if let Ok(msg) = make_msg(file_path.as_ref(), &meta) {
                message = msg;
            }
        }

        display_popup("Oh no :(", message, MessageBoxIcon::Error);
    }));
}

#[allow(unused)]
pub fn make_msg<P: AsRef<Path>>(
    file_path: Option<P>,
    meta: &Metadata,
) -> Result<String, std::fmt::Error> {
    use std::fmt::Write as _;

    let mut buffer = String::new();

    write_msg(&mut buffer, file_path, meta)?;

    Ok(buffer)
}

#[allow(unused)]
fn write_msg<P: AsRef<Path>>(
    buffer: &mut impl std::fmt::Write,
    file_path: Option<P>,
    meta: &Metadata,
) -> std::fmt::Result {
    let (_version, homepage) = (&meta.version, &meta.homepage);

    let name = "Yet Another BG3 Native Mod Loader";

    writeln!(buffer, "Well, this is embarrassing.\n")?;
    writeln!(
        buffer,
        "{name} had a problem and crashed. To help us diagnose the \
     problem, please send us a crash report.\n"
    )?;
    writeln!(
        buffer,
        "We have generated a report file at \"{}\".\n\nSubmit an issue with the subject of \"Crash Report\" and include the report as an attachment. You may also submit the relevant log file found in the plugins directory.\n\nYou can obtain extra trace information by opening a console window, setting env var `YABG3ML_LOG=\"trace\"` and running `./bg3_*.exe --cli` (replace * with the actual tool you want to run)",
        match file_path {
            Some(fp) => format!("{}", fp.as_ref().display()),
            None => "<Failed to store file to disk>".to_string(),
        }
    )?;

    if !homepage.is_empty() {
        writeln!(buffer, "\n{homepage}")?;
    }

    Ok(())
}
