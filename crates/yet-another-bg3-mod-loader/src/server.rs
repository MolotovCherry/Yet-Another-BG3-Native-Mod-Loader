use std::{
    io,
    ops::ControlFlow,
    sync::atomic::{AtomicU32, AtomicU64, Ordering},
};

use shared::{
    pipe::{
        commands::{Level, Receive},
        Server,
    },
    popup::warn_popup,
};
use tracing::{debug, error, info, trace, warn};

pub static AUTH: AtomicU64 = AtomicU64::new(0);
pub static PID: AtomicU32 = AtomicU32::new(0);

pub fn server() -> io::Result<!> {
    let mut server = Server::new();

    let cb = |cmd| match cmd {
        Receive::Log(mut msg) => {
            let filename = msg.filename.unwrap_or_default();
            let line_number = msg.line_number.unwrap_or_default();
            let message = msg.fields.remove("message").unwrap_or_default();
            let target = msg.target;
            let span = msg.span;
            let spans = msg.spans;
            let fields = msg.fields;

            match msg.level {
                Level::Off => (),

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

        Receive::ErrorCantReadPluginDir => {
            warn_popup(
                "Failed to read plugins dir",
                "Attempted to read plugins dir, but failed opening it\n\nDo you have correct perms? See log for more details",
            );
        }
    };

    let auth = |pid, code| {
        let ppid = PID.load(Ordering::Acquire);

        trace!(pid, ppid, "verifying pipe pid");

        if ppid != pid {
            return ControlFlow::Break(());
        }

        let passcode = AUTH.load(Ordering::Acquire);

        trace!(code, passcode, "verifying pipe auth code");

        if passcode == code {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    };

    server.recv_all(cb, auth)?;
}
