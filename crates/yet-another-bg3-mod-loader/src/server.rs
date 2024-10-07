use std::io;

use shared::pipe::{
    commands::{Command, Level},
    Server,
};
use tracing::{debug, error, info, trace, warn};

use crate::popup::warn_popup;

pub fn server() -> io::Result<!> {
    let mut server = Server::new();

    let cb = |cmd| match cmd {
        Command::Log(mut msg) => {
            let filename = msg.filename.unwrap_or_default();
            let line_number = msg.line_number.unwrap_or_default();
            let message = msg.fields.remove("message").unwrap_or_default();
            let target = msg.target;
            let span = msg.span;
            let spans = msg.spans;
            let fields = msg.fields;

            match msg.level {
                Level::Trace => {
                    trace!(target: "loader", %target, %filename, line_number, ?span, ?spans, ?fields, "{message}")
                }

                Level::Debug => {
                    debug!(target: "loader", %target, %filename, line_number, ?span, ?spans, ?fields, "{message}")
                }

                Level::Info => {
                    info!(target: "loader", %target, %filename, line_number, ?span, ?spans, ?fields, "{message}")
                }

                Level::Warn => {
                    warn!(target: "loader", %target, %filename, line_number, ?span, ?spans, ?fields, "{message}")
                }

                Level::Error => {
                    error!(target: "loader", %target, %filename, line_number, ?span, ?spans, ?fields, "{message}")
                }
            }
        }

        Command::ErrorCantReadPluginDir => {
            warn_popup(
                "Failed to read plugins dir",
                "Attempted to read plugins dir, but failed opening it\n\nDo you have correct perms? See log for more details",
            );
        }
    };

    server.recv_all(cb)?;
}
