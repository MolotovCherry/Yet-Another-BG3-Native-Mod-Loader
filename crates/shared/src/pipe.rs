pub mod commands;

use std::{
    cell::RefCell,
    convert::Infallible,
    io::{self, ErrorKind},
    ops::ControlFlow,
    os::windows::prelude::AsRawHandle as _,
    rc::Rc,
    sync::LazyLock,
};

use commands::{Command, Receive};
use eyre::Result;
use serde::Serialize;
use tokio::{
    net::windows::named_pipe::{
        ClientOptions, NamedPipeClient, NamedPipeServer, PipeMode, ServerOptions,
    },
    runtime::{Builder, Runtime},
};
use tracing::{error, trace, trace_span};
use windows::Win32::{
    Foundation::HANDLE,
    Security::{
        InitializeSecurityDescriptor, SetSecurityDescriptorDacl, PSECURITY_DESCRIPTOR,
        SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR,
    },
    System::{Pipes::GetNamedPipeClientProcessId, SystemServices::SECURITY_DESCRIPTOR_REVISION1},
};

use self::commands::Request;

static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to start runtime")
});

pub const PIPE: &str = r"\\.\pipe\yabg3nml";

pub type Pid = u32;
pub type Auth = u64;

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

    pub fn send<T: Serialize>(&self, command: T) -> io::Result<()> {
        let mut buf = Vec::with_capacity(4096);
        // <len><uninit>
        // ^---^ <-- zeroed data
        buf.resize(size_of::<usize>(), 0);

        // <len><message>
        //      ^-------^ <-- add message here
        serde_json::to_writer(&mut buf, &command)?;

        let data_len = buf.len() - size_of::<usize>();
        // <len><message>
        // ^---^ <-- copy len to here
        buf[..size_of::<usize>()].copy_from_slice(&data_len.to_be_bytes());

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

pub struct Server {
    buf: Vec<u8>,
    tbuf: Box<[u8]>,
    msg_len: Option<usize>,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            buf: Vec::with_capacity(4096),
            tbuf: vec![0; 4096].into_boxed_slice(),
            msg_len: None,
        }
    }
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn recv_all(
        &mut self,
        cb: impl Fn(Receive),
        mut auth: impl FnMut(Pid, Auth) -> bool,
    ) -> io::Result<Infallible> {
        let span = trace_span!("pipe");
        let _guard = span.enter();

        // allow all access with security descriptor

        let mut sd = SECURITY_DESCRIPTOR::default();

        unsafe {
            InitializeSecurityDescriptor(
                PSECURITY_DESCRIPTOR(&raw mut sd as *mut _),
                SECURITY_DESCRIPTOR_REVISION1,
            )?;
        }

        unsafe {
            SetSecurityDescriptorDacl(
                PSECURITY_DESCRIPTOR(&raw mut sd as *mut _),
                true,
                None,
                false,
            )?;
        }

        let mut sa = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: &raw mut sd as *mut _,
            bInheritHandle: false.into(),
        };

        let first = Rc::new(RefCell::new(true));
        let mut process_cmd = {
            let first = first.clone();
            move |server: &NamedPipeServer, cmd| {
                let mut first = first.borrow_mut();
                if *first {
                    let span = trace_span!("auth");
                    let _guard = span.enter();

                    #[rustfmt::skip]
                    #[allow(irrefutable_let_patterns)]
                    let Command::Request(Request::Auth(auth_code)) = cmd else {
                        error!(?cmd, "auth not provided, disconnecting client");
                        return ControlFlow::Break(());
                    };

                    trace!(auth_code, "received auth");

                    let handle = HANDLE(server.as_raw_handle());
                    let mut pid = 0;
                    let res = unsafe { GetNamedPipeClientProcessId(handle, &mut pid) };
                    if let Err(e) = res {
                        error!(%e, "failed to get client pid");
                        return ControlFlow::Break(());
                    }

                    if !auth(pid, auth_code) {
                        error!("failed auth, disconnecting");
                        return ControlFlow::Break(());
                    }

                    *first = false;
                } else {
                    let Command::Receive(cmd) = cmd else {
                        error!(?cmd, "did not receive Command::Receive");
                        return ControlFlow::Break(());
                    };

                    let span = trace_span!("cb");
                    let _guard = span.enter();

                    cb(cmd);
                }

                ControlFlow::Continue(())
            }
        };

        let fut = async {
            loop {
                // this lint is returning a false positive?
                #[allow(clippy::multiple_unsafe_ops_per_block)]
                unsafe {
                    self.connect(&mut sa, &mut process_cmd).await?;
                }

                // reset state in case it early exited
                self.buf.clear();
                self.msg_len = None;
                *first.borrow_mut() = true;
            }
        };

        RUNTIME.block_on(fut)
    }

    /// # Safety:
    /// sa must be valid
    async unsafe fn connect(
        &mut self,
        sa: *mut SECURITY_ATTRIBUTES,
        process_cmd: &mut impl FnMut(&NamedPipeServer, Command) -> ControlFlow<()>,
    ) -> Result<(), io::Error> {
        let server = unsafe {
            ServerOptions::new()
                .access_inbound(true)
                .access_outbound(false)
                .reject_remote_clients(true)
                .pipe_mode(PipeMode::Byte)
                .create_with_security_attributes_raw(PIPE, sa.cast())
        };

        let server = match server {
            Ok(s) => s,
            Err(e) => {
                error!(%e, "failed to create server");
                return Err(e);
            }
        };

        if let Err(e) = server.connect().await {
            error!(%e, "client failed to connect");
            // not an error in the sense that it's not fatal
            return Ok(());
        }

        loop {
            if server.readable().await.is_err() {
                break;
            }

            match server.try_read(&mut self.tbuf) {
                Ok(0) => break,

                Ok(n) => {
                    let data = &self.tbuf[..n];
                    self.buf.extend_from_slice(data);

                    // message: <len:usize><message>
                    // this will keep looping and process each msg len / message for as long as there's enough
                    // data buffered
                    loop {
                        // 1. get msg len if it's not set and we have enough buffer to get it
                        // <len><message>
                        // ^---^
                        if self.msg_len.is_none() && self.buf.len() >= size_of::<usize>() {
                            self.msg_len = Some(usize::from_be_bytes(
                                self.buf[..size_of::<usize>()].try_into().unwrap(),
                            ));

                            continue;
                        }
                        // 2. process msg if we know the msg len and there's enough buffer to process it
                        // <len><message>
                        //      ^-------^
                        else if let Some(len) = self.msg_len {
                            if self.buf.len() < len + size_of::<usize>() {
                                break;
                            }

                            let data = &self.buf[size_of::<usize>()..len + size_of::<usize>()];

                            let Ok(cmd) = serde_json::from_slice::<Command>(data) else {
                                trace!(?data, "received invalid cmd");
                                return Ok(());
                            };

                            if process_cmd(&server, cmd).is_break() {
                                _ = server.disconnect();
                                return Ok(());
                            }

                            self.buf.drain(..len + size_of::<usize>());
                            self.msg_len = None;

                            continue;
                        }

                        // there's no len or message left to process
                        break;
                    }

                    continue;
                }

                Err(e) if e.kind() == ErrorKind::WouldBlock => continue,

                Err(e) => {
                    error!(%e, "client error");
                    break;
                }
            }
        }

        Ok(())
    }
}
