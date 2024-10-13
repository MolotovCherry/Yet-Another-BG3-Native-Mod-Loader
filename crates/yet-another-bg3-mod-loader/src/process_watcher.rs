use std::{
    collections::HashSet,
    mem::ManuallyDrop,
    sync::{
        mpsc::{channel, Receiver, RecvTimeoutError, Sender},
        Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use eyre::{Error, Result};
use tracing::{trace, trace_span};
use unicase::UniCase;
use windows::Win32::{
    Foundation::MAX_PATH,
    System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
};

use crate::{
    helpers::OwnedHandle,
    wapi::{
        get_processes::get_processes,
        query_full_process_image_name::query_full_process_image_name_w,
    },
};

pub type Pid = u32;

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

#[derive(Debug)]
pub struct ProcessWatcherWaiter {
    thread: Mutex<Option<JoinHandle<Result<()>>>>,
    wait_receiver: Receiver<()>,
}

impl ProcessWatcherWaiter {
    pub fn wait(&self) {
        trace!("process watcher waiting");
        _ = self.wait_receiver.recv();

        trace!("process watcher thread joining");
        _ = self.thread.lock().unwrap().take().unwrap().join();

        trace!("process watcher finished wait");
    }
}

#[derive(Debug)]
pub struct ProcessWatcherStopToken {
    thread_sender: ManuallyDrop<Sender<()>>,
    wait_sender: ManuallyDrop<Sender<()>>,
}

impl ProcessWatcherStopToken {
    pub fn stop(&self) {
        trace!("process watcher stop token stopping");

        // this may fail if the thread exited early, but that doesn't matter at this point
        _ = self.thread_sender.send(());

        self.wait_sender.send(()).unwrap();
    }
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
    pub fn new<S: AsRef<str> + Clone>(
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

    pub fn run(
        mut self,
        cb: impl Fn(CallType) + Send + 'static,
    ) -> (ProcessWatcherWaiter, ProcessWatcherStopToken) {
        let (thread_sender, thread_receiver) = channel();
        let (wait_sender, wait_receiver) = channel();

        let wait_sender_clone = wait_sender.clone();

        let thread = thread::spawn(move || {
            // we can avoid unsafe length setting shenanigans by prefilling it, instead of set_len
            let mut pid_buf = vec![0u32; 1024];
            let mut new_pid_buf = vec![0u32; 1024];
            // important to prefill it, that way len() returns the full amount for any ffi calls
            let mut path_buf = vec![0u16; MAX_PATH as usize];

            let mut now = None;
            let mut end = None;

            if let Timeout::Duration(d) = self.timeout {
                let inst = Instant::now();

                now = Some(inst);
                end = Some(d);
            }

            'run: loop {
                if let Some(now) = now {
                    trace!("initiating timeout check");

                    if now.elapsed() >= end.unwrap() {
                        trace!("detected a timeout");

                        cb(CallType::Timeout);

                        if self.oneshot {
                            trace!("initiating oneshot channel event");
                            _ = wait_sender_clone.send(());
                        }

                        break 'run;
                    }
                }

                let pids = get_processes(&mut pid_buf);

                // process list of pids, compare to last cached copy, find new ones and process those
                self.process_pids(pids, &mut new_pid_buf);

                'pid_loop: for pid in new_pid_buf.iter().copied() {
                    let span_pid_loop = trace_span!("pid_loop", pid = pid);
                    let _guard = span_pid_loop.enter();

                    let process = {
                        let res =
                            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) };

                        match res {
                            Ok(v) => OwnedHandle::new(v),
                            Err(e) => {
                                // failed to open process; probably we don't have correct perms to open it
                                // there is a risk here that we don't have permission to open the game process, so it's skipped
                                // in such a case, this tool should be run as admin. we have no way of knowing if that happened

                                trace!(err = %e, "failed to open process");

                                continue;
                            }
                        }
                    };

                    let Ok(path) = query_full_process_image_name_w(&process, &mut path_buf) else {
                        continue;
                    };

                    let new_process_path = UniCase::new(path.to_string_lossy());

                    trace!(path = %new_process_path, "found process");

                    for process_path in &self.processes {
                        if process_path == &new_process_path {
                            trace!(path = %process_path, "found process match");

                            cb(CallType::Pid(pid));

                            if self.oneshot {
                                trace!("we're on oneshot. sending event..");
                                _ = wait_sender_clone.send(());
                                break 'run;
                            }

                            // there can only be one match per pid, so..
                            continue 'pid_loop;
                        }
                    }
                }

                let signal = thread_receiver.recv_timeout(self.polling_rate);

                if matches!(signal, Ok(_) | Err(RecvTimeoutError::Disconnected)) {
                    trace!(?signal, "signal thread_receiver exited");
                    break 'run;
                }
            }

            Ok::<_, Error>(())
        });

        let waiter = ProcessWatcherWaiter {
            thread: Mutex::new(Some(thread)),
            wait_receiver,
        };

        let token = ProcessWatcherStopToken {
            thread_sender: ManuallyDrop::new(thread_sender),
            wait_sender: ManuallyDrop::new(wait_sender),
        };

        (waiter, token)
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
