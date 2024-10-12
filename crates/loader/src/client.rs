use std::{io, sync::LazyLock};

use serde::Serialize;
use shared::pipe::Client;

pub static CLIENT: LazyLock<Result<Client, io::Error>> = LazyLock::new(Client::new);

pub trait TrySend {
    fn try_send<T: Serialize>(&self, command: T) -> Result<(), io::Error>;
}

impl TrySend for Result<Client, io::Error> {
    /// try to send, but noop if not connected
    fn try_send<T: Serialize>(&self, command: T) -> Result<(), io::Error> {
        match self {
            Ok(c) => c.send(command),
            // noop if not connected
            Err(_) => Ok(()),
        }
    }
}
