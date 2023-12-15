use std::{
    collections::HashSet,
    mem::ManuallyDrop,
    os::windows::ffi::OsStrExt,
    path::PathBuf,
    sync::{
        mpsc::{channel, Receiver, RecvTimeoutError, Sender},
        Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use windows::Win32::{
    Foundation::MAX_PATH,
    System::{
        ProcessStatus::{EnumProcesses, GetModuleFileNameExW},
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    },
};

use crate::helpers::OwnedHandle;

#[derive(Debug)]
pub enum CallType {
    Pid(u32),
    Timeout,
}

#[derive(Debug, PartialEq)]
pub enum Timeout {
    None,
    Duration(Duration),
}

#[derive(Debug)]
pub struct ProcessWatcherWaiter {
    thread: Mutex<Option<JoinHandle<anyhow::Result<()>>>>,
    wait_receiver: Receiver<()>,
}

impl ProcessWatcherWaiter {
    pub fn wait(&self) {
        _ = self.wait_receiver.recv();

        self.thread
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .join()
            .unwrap()
            .unwrap();
    }
}

#[derive(Debug)]
pub struct ProcessWatcherStopToken {
    thread_sender: ManuallyDrop<Sender<()>>,
    wait_sender: ManuallyDrop<Sender<()>>,
}

impl ProcessWatcherStopToken {
    pub fn stop(&self) {
        self.thread_sender.send(()).unwrap();
        self.wait_sender.send(()).unwrap();
    }
}

#[derive(Debug)]
pub struct ProcessWatcher {
    processes: Vec<Vec<u16>>,
    polling_rate: Duration,
    timeout: Timeout,
    state: HashSet<u32>,
    oneshot: bool,
}

impl ProcessWatcher {
    /// timeout is in ms
    /// processes must be full path to exe
    pub fn new<P: Into<PathBuf>>(
        processes: Vec<P>,
        polling_rate: Duration,
        timeout: Timeout,
        oneshot: bool,
    ) -> Self {
        Self {
            processes: processes
                .into_iter()
                .map(|p| p.into().as_os_str().encode_wide().collect())
                .collect(),
            state: HashSet::new(),
            polling_rate,
            timeout,
            oneshot,
        }
    }

    pub fn run(
        mut self,
        callback: impl Fn(CallType) + Send + 'static,
    ) -> (ProcessWatcherWaiter, ProcessWatcherStopToken) {
        let (thread_sender, thread_receiver) = channel();
        let (wait_sender, wait_receiver) = channel();

        let wait_sender_clone = wait_sender.clone();

        let thread = thread::spawn(move || {
            // we can avoid unsafe length setting shenanigans by prefilling it, instead of set_len
            let mut pid_buffer = vec![0u32; 1024];
            let mut new_pid_buffer = vec![0u32; 1024];
            // important to prefill it, that way len() returns the full amount for any ffi calls
            let mut path_buffer = vec![0u16; MAX_PATH as usize];

            let mut lpcneeded = 0;

            let mut now = None;
            let mut end = None;

            if let Timeout::Duration(d) = self.timeout {
                let inst = Instant::now();

                now = Some(inst);
                end = Some(d);
            }

            'run: loop {
                if let Some(now) = now {
                    if now.elapsed() >= end.unwrap() {
                        callback(CallType::Timeout);

                        if self.oneshot {
                            _ = wait_sender_clone.send(());
                        }

                        break 'run;
                    }
                }

                let cb = (pid_buffer.capacity() * 4).try_into().unwrap();

                unsafe {
                    EnumProcesses(pid_buffer.as_mut_ptr(), cb, &mut lpcneeded)?;
                }

                // if lpcbNeeded equals cb, consider retrying the call with a larger array
                //
                // this intentionally keeps growing until it has enough capacity, and never shrinks itself
                if lpcneeded == cb {
                    pid_buffer.resize(pid_buffer.capacity() + 1024, 0);
                    continue 'run;
                }

                // To determine how many processes were enumerated, divide the lpcbNeeded value by sizeof(DWORD).
                let num_processes = (lpcneeded / 4) as usize;
                let pids = &pid_buffer[..num_processes];

                // process list of pids, compare to last cached copy, find new ones and process those
                self.process_pids(pids, &mut new_pid_buffer);

                'pid_loop: for pid in new_pid_buffer.iter().copied() {
                    let handle_res: Result<OwnedHandle, _> = unsafe {
                        OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
                            .map(|h| h.into())
                    };

                    let Ok(handle) = handle_res else {
                        // failed to open process; probably we don't have correct perms to open it
                        continue;
                    };

                    let written = unsafe {
                        GetModuleFileNameExW(handle.as_raw_handle(), None, &mut path_buffer)
                            as usize
                    };

                    if written == 0 {
                        continue;
                    }

                    let new_process_path = &path_buffer[..written];

                    for process_path in &self.processes {
                        if process_path == new_process_path {
                            callback(CallType::Pid(pid));

                            if self.oneshot {
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
                    break 'run;
                }
            }

            Ok::<_, anyhow::Error>(())
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
        buffer.clear();

        for &pid in pids {
            if self.state.insert(pid) {
                buffer.push(pid);
            }
        }

        // this is important. It erases all the old entries in the table
        // clear the table to keep backing memory
        self.state.clear();
        // and re-extend it
        self.state.extend(pids);
    }
}
