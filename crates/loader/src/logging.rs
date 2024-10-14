use std::{
    io::{self, ErrorKind, Write},
    sync::{Mutex, MutexGuard},
};

use eyre::Result;
use shared::{pipe::commands::Receive, thread_data::LogData, utils::SuperLock as _};
use tracing_subscriber::{fmt::MakeWriter, util::SubscriberInitExt};

use crate::client::{TrySend, CLIENT};

pub fn setup_logging(data: &LogData) -> Result<()> {
    let maker = PipeMaker::new()?;

    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_file(true)
        .json()
        .with_max_level(data.level)
        .with_writer(maker)
        .with_target(data.target)
        .finish()
        .init();

    Ok(())
}

struct PipeMaker {
    buf: Mutex<Vec<u8>>,
}

impl PipeMaker {
    fn new() -> io::Result<Self> {
        let s = Self {
            buf: Mutex::new(Vec::with_capacity(4096)),
        };

        Ok(s)
    }
}

impl<'a> MakeWriter<'a> for PipeMaker {
    type Writer = PipeWriter<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        Self::Writer { buf: &self.buf }
    }
}

struct BufClear<'a>(MutexGuard<'a, Vec<u8>>);

impl Drop for BufClear<'_> {
    fn drop(&mut self) {
        self.0.clear();
    }
}

struct PipeWriter<'a> {
    buf: &'a Mutex<Vec<u8>>,
}

impl Write for PipeWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // noop write
        if CLIENT.is_err() {
            return Ok(buf.len());
        }

        let mut v = self.buf.super_lock();
        v.extend(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // noop flush
        if CLIENT.is_err() {
            return Ok(());
        }

        let v = BufClear(self.buf.super_lock());

        let data =
            v.0.as_slice()
                .try_into()
                .map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;

        let c = Receive::Log(data);

        CLIENT.try_send(c)?;

        Ok(())
    }
}

impl Drop for PipeWriter<'_> {
    fn drop(&mut self) {
        _ = self.flush();
    }
}
