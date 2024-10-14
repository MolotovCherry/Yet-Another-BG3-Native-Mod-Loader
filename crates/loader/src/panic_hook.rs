use std::panic;

use tracing::error;

pub fn set_hook() {
    panic::set_hook(Box::new(move |info| {
        #[allow(unused_assignments, unused_mut)]
        let mut message = info.to_string();

        // For debug mode, print entire stack trace. Stack trace doesn't really
        // contain anything useful in release mode due to optimizations
        #[cfg(debug_assertions)]
        {
            use shared::backtrace::CaptureBacktrace;
            message = format!("{info}\n\nstack backtrace:\n{}", CaptureBacktrace);
        }

        // Dump panic info
        error!("{message}");
    }));
}
