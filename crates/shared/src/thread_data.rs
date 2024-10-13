use serde::{Deserialize, Serialize};

use crate::pipe::commands::Level;

#[derive(Serialize, Deserialize)]
pub struct ThreadData {
    /// the authentication code
    pub auth: u64,
    /// the log level to start at
    pub level: Level,
}
