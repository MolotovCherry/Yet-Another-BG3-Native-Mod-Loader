use crate::pipe::commands::Level;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ThreadData {
    /// the authentication code
    pub auth: u64,
    // log data
    pub log: LogData,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct LogData {
    /// the log level to start at
    pub level: Level,
    /// whether to enable targets
    pub target: bool,
}
