use std::fs::File;

use eyre::Result;
use log::{error, LevelFilter};
#[allow(unused)]
use simplelog::{
    ColorChoice, CombinedLogger, Config, SharedLogger, TermLogger, TerminalMode, WriteLogger,
};
use windows::Win32::Foundation::HINSTANCE;

use crate::paths::get_dll_dir;

pub fn setup_logging(module: HINSTANCE) -> Result<()> {
    // get the file path to `<path_to_my_dll_folder>\`
    let dll_dir = get_dll_dir(module)?;
    let log_path = dll_dir.join("yet-another-bg3-mod-loader.log");

    let loggers: Vec<Box<dyn SharedLogger>> = vec![
        WriteLogger::new(
            if cfg!(debug_assertions) {
                LevelFilter::Trace
            } else {
                LevelFilter::Info
            },
            Config::default(),
            File::create(log_path).unwrap(),
        ),
        #[cfg(debug_assertions)]
        TermLogger::new(
            LevelFilter::Trace,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::AlwaysAnsi,
        ),
    ];

    CombinedLogger::init(loggers)?;

    set_panic_hook();

    Ok(())
}

fn set_panic_hook() {
    // this panic hook makes sure that eyre panic hook gets sent to all tracing layers
    std::panic::set_hook(Box::new(move |panic| {
        let panic = panic.to_string();

        error!("{panic}");

        #[cfg(not(any(debug_assertions, feature = "console")))]
        crate::popup::fatal_popup(
            "Yet Another BG3 Mod Loader Panic",
            format!("The mod loader unexpectedly crashed. Please consider reporting the bug @ https://github.com/MolotovCherry/Yet-Another-BG3-Native-Mod-Loader\n\n{panic}"),
        );
    }))
}
