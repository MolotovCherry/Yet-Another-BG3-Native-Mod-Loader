use clap::Parser;

/// A simple, non-invasive BG3 native mod loader
#[derive(Parser, Debug)]
#[clap(disable_help_flag = true, disable_version_flag = true)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Show console window
    #[arg(long)]
    pub cli: bool,

    /// Print help
    #[arg(short, long, conflicts_with = "version")]
    pub help: bool,

    /// Print version
    #[arg(short = 'V', long, conflicts_with = "help")]
    pub version: bool,

    /// Binary to test inject
    #[cfg(feature = "test-injection")]
    #[arg(long, required = true)]
    pub inject: String,
}
