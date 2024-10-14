use shared::utils::SuperLock as _;
use tracing::{error, trace, trace_span};
use windows::Win32::System::ProcessStatus::EnumProcesses;

use crate::process_watcher::{Pid, CURRENT_PID};

#[allow(non_snake_case)]
pub fn EnumProcessesRs(buf: &mut Vec<Pid>) -> &[Pid] {
    let span = trace_span!(parent: CURRENT_PID.super_lock().clone(), "EnumProcessesRs");
    let _guard = span.enter();

    let mut lpcbneeded = 0;

    loop {
        let size = (buf.len() * size_of::<Pid>()) as u32;

        let enum_res = unsafe { EnumProcesses(buf.as_mut_ptr(), size, &mut lpcbneeded) };

        // There is no indication given when the buffer is too small to store all process identifiers. Therefore, if lpcbNeeded
        // equals cb, consider retrying the call with a larger array.
        if lpcbneeded == size {
            let new_len = buf.len() + 1024;

            trace!(
                lpcbneeded,
                size,
                new_len,
                "lpcbneeded == cb; pid_buffer not large enough; increasing size",
            );

            buf.resize(new_len, 0);
            continue;
        }

        if let Err(e) = enum_res {
            error!("{e}");
            continue;
        }

        let n_processes = (lpcbneeded / size_of::<Pid>() as u32) as usize;
        break &buf[..n_processes];
    }
}
