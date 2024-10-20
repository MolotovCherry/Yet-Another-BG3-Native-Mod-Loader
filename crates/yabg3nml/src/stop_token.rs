use std::sync::mpsc::Sender;

use tracing::trace;

#[derive(Debug)]
pub struct StopToken {
    signal: Sender<()>,
}

impl StopToken {
    pub fn new(signal: Sender<()>) -> Self {
        Self { signal }
    }

    pub fn stop(&self) {
        trace!("stop token stopping");
        // this may fail if the thread exited early, but that doesn't matter at this point
        _ = self.signal.send(());
    }
}
