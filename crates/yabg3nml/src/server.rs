use std::{
    convert::Infallible,
    io,
    ops::ControlFlow,
    sync::atomic::{AtomicU32, AtomicU64, Ordering},
};

use shared::pipe::{
    commands::{Level, Receive},
    Server,
};
use tracing::{debug, error, info, trace, trace_span, warn};

pub static AUTH: AtomicU64 = AtomicU64::new(0);
pub static PID: AtomicU32 = AtomicU32::new(0);

pub fn server() -> io::Result<Infallible> {
    let mut server = Server::new();

    let cb = |cmd| {
        let span = trace_span!("dll");
        let _guard = span.enter();

        match cmd {
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
                        trace!(target: "loader", ?target, %filename, line_number, ?span, ?spans, ?fields, "{message}")
                    }

                    Level::Debug => {
                        debug!(target: "loader", ?target, %filename, line_number, ?span, ?spans, ?fields, "{message}")
                    }

                    Level::Info => {
                        info!(target: "loader", ?target, %filename, line_number, ?span, ?spans, ?fields, "{message}")
                    }

                    Level::Warn => {
                        warn!(target: "loader", ?target, %filename, line_number, ?span, ?spans, ?fields, "{message}")
                    }

                    Level::Error => {
                        error!(target: "loader", ?target, %filename, line_number, ?span, ?spans, ?fields, "{message}")
                    }
                }
            }
        }
    };

    let auth = |pid, code| {
        let ppid = PID.load(Ordering::Relaxed);

        trace!(pid, ppid, "verifying pid");

        if ppid != pid {
            return ControlFlow::Break(());
        }

        // load current auth code; also change auth code to always keep it randomized
        let passcode = AUTH.swap(rand::random::<u64>(), Ordering::Relaxed);

        trace!(code, passcode, "verifying auth code");

        if passcode == code {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    };

    server.recv_all(cb, auth)
}
