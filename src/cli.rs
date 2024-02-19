use clap::Parser;

/// A simple, non-invasive injector for BG3 native dll plugins
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Show console window
    #[arg(long)]
    pub cli: bool,
}
