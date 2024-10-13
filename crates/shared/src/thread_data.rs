use crate::pipe::commands::Level;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ThreadData {
    /// the authentication code
    pub auth: u64,
    /// the log level to start at
    pub level: Level,
}
