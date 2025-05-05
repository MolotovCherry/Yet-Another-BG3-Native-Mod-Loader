use std::{
    collections::HashSet,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{RecvTimeoutError, channel},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use shared::utils::{OwnedHandle, SuperLock};
use tracing::{Span, trace, trace_span};
use unicase::UniCase;
use windows::Win32::{
    Foundation::MAX_PATH,
    System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION},
};

use crate::{
    stop_token::StopToken,
    wapi::{
        enum_processes::EnumProcessesRs, query_full_process_image_name::QueryFullProcessImageNameRs,
    },
};

pub type Pid = u32;

pub static CURRENT_PID: LazyLock<Mutex<Span>> = LazyLock::new(|| Mutex::new(Span::none()));

#[derive(Debug)]
pub enum CallType {
    Pid(Pid),
    Timeout,
}

#[derive(Debug, PartialEq)]
pub enum Timeout {
    None,
    Duration(Duration),
}

impl Timeout {
    fn is_timeout(&self) -> bool {
        matches!(self, Self::Duration(_))
    }
}

#[derive(Debug)]
pub struct ProcessWatcherResults {
    pub watcher_token: StopToken,
    pub timeout_token: Option<StopToken>,
    pub watcher_handle: JoinHandle<()>,
}

#[derive(Debug)]
pub struct ProcessWatcher {
    processes: Vec<UniCase<String>>,
    polling_rate: Duration,
    timeout: Timeout,
    state: HashSet<u32>,
    oneshot: bool,
}

impl ProcessWatcher {
    /// timeout is in ms
    /// processes must be full path to exe
    pub fn new<S: AsRef<str>>(
        processes: &[S],
        polling_rate: Duration,
        timeout: Timeout,
        oneshot: bool,
    ) -> Self {
        Self {
            processes: processes
                .iter()
                .map(|p| UniCase::new(p.as_ref().to_owned()))
                .collect(),
            state: HashSet::new(),
            polling_rate,
            timeout,
            oneshot,
        }
    }

    pub fn run(mut self, cb: impl Fn(CallType) + Send + Sync + 'static) -> ProcessWatcherResults {
        let (sender, recv) = channel();
        let timed_out = Arc::new(AtomicBool::new(false));

        let timeout_token = if let Timeout::Duration(d) = self.timeout {
            let now = Instant::now();
            let end = d;

            let (tt_sender, tt_recv) = channel();

            let timed_out = timed_out.clone();
            let sender = sender.clone();

            thread::spawn(move || {
                loop {
                    let signal = tt_recv.recv_timeout(Duration::from_secs(1));
                    if matches!(signal, Ok(_) | Err(RecvTimeoutError::Disconnected)) {
                        break;
                    }

                    // handle timeout
                    if now.elapsed() >= end {
                        trace!("detected a timeout");

                        timed_out.store(true, Ordering::Relaxed);
                        _ = sender.send(());
                        break;
                    }
                }
            });

            Some(StopToken::new(tt_sender))
        } else {
            None
        };

        let handle = thread::spawn(move || {
            // we can avoid unsafe length setting shenanigans by prefilling it, instead of set_len
            let mut pid_buf = vec![0u32; 1024];
            let mut new_pid_buf = vec![0u32; 1024];
            // important to prefill it, that way len() returns the full amount for any ffi calls
            let mut path_buf = vec![0u16; MAX_PATH as usize];

            'run: loop {
                let pids = EnumProcessesRs(&mut pid_buf);

                // process list of pids, compare to last cached copy, find new ones and process those
                self.process_pids(pids, &mut new_pid_buf);

                'pid_loop: for pid in new_pid_buf.iter().copied() {
                    let span_pid_loop = trace_span!("pid_loop", pid = pid);
                    let _guard = span_pid_loop.enter();

                    *CURRENT_PID.super_lock() = span_pid_loop.clone();

                    let process = {
                        let res = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION, false, pid) };

                        match res {
                            Ok(v) => unsafe { OwnedHandle::new(v) },
                            Err(e) => {
                                // failed to open process; probably we don't have correct perms to open it
                                // there is a risk here that we don't have permission to open the game process, so it's skipped
                                // in such a case, this tool should be run as admin. we have no way of knowing if that happened

                                trace!(err = %e, "failed to open process");

                                continue;
                            }
                        }
                    };

                    let Ok(path) = QueryFullProcessImageNameRs(&process, &mut path_buf) else {
                        continue;
                    };

                    let new_process_path = UniCase::new(path.to_string_lossy());

                    trace!(process = %new_process_path, "found");

                    for process_path in &self.processes {
                        if process_path == &new_process_path {
                            trace!(path = %process_path, "found process match");

                            cb(CallType::Pid(pid));

                            if self.oneshot {
                                break 'run;
                            }

                            // there can only be one match per pid, so..
                            continue 'pid_loop;
                        }
                    }
                }

                let signal = recv.recv_timeout(self.polling_rate);
                if matches!(signal, Ok(_) | Err(RecvTimeoutError::Disconnected)) {
                    trace!(?signal, "signal exited");

                    // if we have a timeout running and it timed out, wait before quitting
                    if self.timeout.is_timeout() && timed_out.load(Ordering::Relaxed) {
                        cb(CallType::Timeout);
                    }

                    break;
                }
            }
        });

        let watcher_token = StopToken::new(sender);

        ProcessWatcherResults {
            watcher_token,
            timeout_token,
            watcher_handle: handle,
        }
    }

    /// processes pids and detects which processes are new
    ///
    /// buffer is the working memory we'll use to store the new pid results in
    fn process_pids(&mut self, pids: &[u32], buffer: &mut Vec<u32>) {
        let span = trace_span!("process_pids");
        let _guard = span.enter();

        buffer.clear();

        for &pid in pids {
            if self.state.insert(pid) {
                buffer.push(pid);
            }
        }

        if !buffer.is_empty() {
            trace!(pids = ?buffer, "found new pids to check");
        }

        // this is important. It erases all the old entries in the table
        // clear the table to keep backing memory
        self.state.clear();
        // and re-extend it
        self.state.extend(pids);
    }
}
