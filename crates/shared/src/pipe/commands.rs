use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Log(LogMsg),
    ErrorCantReadPluginDir,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogMsg {
    pub level: Level,
    pub target: String,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<Level> for tracing::Level {
    fn from(value: Level) -> Self {
        match value {
            Level::Trace => tracing::Level::TRACE,
            Level::Debug => tracing::Level::DEBUG,
            Level::Info => tracing::Level::INFO,
            Level::Warn => tracing::Level::WARN,
            Level::Error => tracing::Level::ERROR,
        }
    }
}

impl TryFrom<&[u8]> for LogMsg {
    type Error = serde_json::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(value)
    }
}
