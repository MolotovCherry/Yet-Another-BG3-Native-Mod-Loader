use argh::FromArgs;

/// A simple, non-invasive BG3 native mod loader
#[allow(unused)]
#[derive(Default, FromArgs)]
pub struct Args {
    /// binary to test inject
    #[cfg(feature = "test-injection")]
    #[argh(option)]
    pub inject: String,
}
