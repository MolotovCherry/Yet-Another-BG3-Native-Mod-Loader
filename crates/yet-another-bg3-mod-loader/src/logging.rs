use std::path::Path;

use eyre::Result;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

use crate::cli::Args;

pub fn setup_logs<P: AsRef<Path>>(plugins_dir: P, args: &Args) -> Result<Option<WorkerGuard>> {
    let mut worker_guard: Option<WorkerGuard> = None;

    if cfg!(debug_assertions) || args.cli {
        #[cfg(not(debug_assertions))]
        {
            use crate::console::debug_console;
            debug_console("Yet Another BG3 Native Mod Loader Debug Console")?;
        }

        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_env("YABG3ML_LOG"))
            .without_time()
            .init();
    } else {
        let plugins_dir = plugins_dir.as_ref();
        let logs_dir = plugins_dir.join("logs");

        let file_appender = tracing_appender::rolling::daily(logs_dir, "ya-native-mod-loader");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        worker_guard = Some(_guard);
        tracing_subscriber::fmt()
            .with_max_level(LevelFilter::DEBUG)
            .with_writer(non_blocking)
            .with_ansi(false)
            .init();
    }

    Ok(worker_guard)
}
