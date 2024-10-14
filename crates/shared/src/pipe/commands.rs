use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};
use tracing::level_filters::LevelFilter;

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Auth(u64),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Receive {
    Log(LogMsg),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogMsg {
    pub level: Level,
    pub target: Option<String>,
    pub filename: Option<String>,
    pub line_number: Option<u32>,
    pub span: Option<Span>,
    pub spans: Option<Vec<Span>>,
    pub fields: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Span {
    pub name: String,
}

impl Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Level {
    Off,
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LevelFilter> for Level {
    fn from(value: LevelFilter) -> Self {
        match value.into_level() {
            Some(l) => match l {
                tracing::Level::TRACE => Self::Trace,
                tracing::Level::DEBUG => Self::Debug,
                tracing::Level::INFO => Self::Info,
                tracing::Level::WARN => Self::Warn,
                tracing::Level::ERROR => Self::Error,
            },

            None => Self::Off,
        }
    }
}

impl From<Level> for LevelFilter {
    fn from(value: Level) -> Self {
        match value {
            Level::Off => Self::OFF,
            Level::Trace => Self::TRACE,
            Level::Debug => Self::DEBUG,
            Level::Info => Self::INFO,
            Level::Warn => Self::WARN,
            Level::Error => Self::ERROR,
        }
    }
}

impl TryFrom<&[u8]> for LogMsg {
    type Error = serde_json::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(value)
    }
}
