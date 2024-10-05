pub mod commands;

use std::{
    io::{self, ErrorKind},
    sync::LazyLock,
};

use commands::Command;
use tokio::{
    net::windows::named_pipe::{ClientOptions, NamedPipeClient, PipeMode, ServerOptions},
    runtime::{Builder, Runtime},
};

static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to start runtime")
});

pub const PIPE: &str = r"\\.\pipe\yabg3ml";

pub struct Client {
    pipe: NamedPipeClient,
}

impl Client {
    pub fn new() -> io::Result<Self> {
        let fut = async {
            ClientOptions::new()
                .read(false)
                .write(true)
                .pipe_mode(PipeMode::Byte)
                .open(PIPE)
        };

        let pipe = RUNTIME.block_on(fut)?;

        Ok(Self { pipe })
    }

    pub fn send(&self, command: Command) -> io::Result<()> {
        let mut buf = Vec::with_capacity(4096);
        // <len><..uninit>
        // ^---^ <-- zeroed data
        buf.resize(size_of::<usize>(), 0);

        // <len><message>
        //      ^-------^ <-- add message here
        serde_json::to_writer(&mut buf, &command)?;

        let data_len = buf.len() - size_of::<usize>();
        // <len><message>
        // ^---^ <-- copy len to here
        buf[..size_of::<usize>()].copy_from_slice(&data_len.to_le_bytes());

        let fut = async {
            let size = buf.len();
            let mut pos = 0;

            loop {
                self.pipe.writable().await?;

                match self.pipe.try_write(&buf[pos..]) {
                    Ok(n) => {
                        pos += n;

                        if pos >= size {
                            break;
                        }

                        continue;
                    }

                    Err(e) if e.kind() == ErrorKind::WouldBlock => continue,

                    Err(e) => return Err(e),
                }
            }

            Ok(())
        };

        RUNTIME.block_on(fut)?;
        Ok(())
    }
}

pub struct Server;

impl Server {
    pub fn new() -> Self {
        Self
    }

    pub fn recv(&self, cb: impl Fn(Command)) -> io::Result<()> {
        let fut = async {
            let mut buf = Vec::with_capacity(4096);
            let mut tbuf = [0; 1024];
            let mut msg_len = 0;

            let server = ServerOptions::new()
                .access_inbound(true)
                .access_outbound(false)
                .reject_remote_clients(true)
                .pipe_mode(PipeMode::Byte)
                .create(PIPE)?;

            server.connect().await?;

            loop {
                if server.readable().await.is_err() {
                    break;
                }

                match server.try_read(&mut tbuf) {
                    Ok(0) => break,

                    Ok(n) => {
                        let data = &tbuf[..n];
                        buf.extend_from_slice(data);

                        // message: <len:usize><message>
                        // this will keep looping and process each msg len / message for as long as there's enough
                        // data buffered
                        loop {
                            // 1. get msg len if it's not set and we have enough buffer to get it
                            // <len><message>
                            // ^---^
                            if msg_len == 0 && buf.len() >= size_of::<usize>() {
                                msg_len = usize::from_le_bytes(
                                    buf[..size_of::<usize>()].try_into().unwrap(),
                                );

                                continue;
                            // 2. process msg if we know the msg len and there's enough buffer to process it
                            // <len><message>
                            //      ^-------^
                            } else if msg_len > 0 && buf.len() >= msg_len + size_of::<usize>() {
                                let data = &buf[size_of::<usize>()..msg_len + size_of::<usize>()];

                                if let Ok(command) = serde_json::from_slice::<Command>(data) {
                                    cb(command);
                                }

                                buf.drain(..msg_len + size_of::<usize>());
                                msg_len = 0;

                                continue;
                            }

                            // there's no len or message left to process
                            break;
                        }

                        continue;
                    }

                    Err(e) if e.kind() == ErrorKind::WouldBlock => continue,

                    Err(e) => return Err(e),
                }
            }

            Ok(())
        };

        RUNTIME.block_on(fut)?;

        Ok(())
    }
}