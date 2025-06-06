use std::{env, path::Path};

use eyre::Result;
use shared::config::Config;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

pub fn setup_logs<P: AsRef<Path>>(config: &Config, plugins_dir: P) -> Result<Option<WorkerGuard>> {
    let mut worker_guard: Option<WorkerGuard> = None;

    // env var takes precedence over config value
    let env = env::var("YABG3NML_LOG");
    let env = env.as_deref().unwrap_or(&config.log.level);
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse_lossy(env);

    if cfg!(debug_assertions) || config.core.cli {
        #[cfg(not(debug_assertions))]
        {
            use crate::console::debug_console;
            debug_console("Yet Another BG3 Native Mod Loader Debug Console")?;
        }

        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(config.log.target)
            .without_time()
            .init();
    } else {
        let plugins_dir = plugins_dir.as_ref();
        let logs_dir = plugins_dir.join("logs");

        let file_appender = tracing_appender::rolling::daily(logs_dir, "ya-bg3-native-mod-loader");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        worker_guard = Some(_guard);
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(non_blocking)
            .with_target(config.log.target)
            .without_time()
            .with_ansi(false)
            .init();
    }

    Ok(worker_guard)
}
