use argh::FromArgs;

/// A simple, non-invasive BG3 native mod loader
#[derive(FromArgs)]
pub struct Args {
    /// show console window
    #[argh(switch)]
    pub cli: bool,

    /// binary to test inject
    #[cfg(feature = "test-injection")]
    #[argh(option)]
    pub inject: String,
}
