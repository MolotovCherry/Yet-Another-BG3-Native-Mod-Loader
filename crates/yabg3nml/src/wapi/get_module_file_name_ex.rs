use eyre::{Result, bail};
use shared::utils::OwnedHandle;
use tracing::{error, trace, trace_span};
use widestring::U16Str;
use windows::Win32::{
    Foundation::{GetLastError, HMODULE, MAX_PATH},
    System::ProcessStatus::GetModuleFileNameExW,
};

use crate::process_watcher::CURRENT_PID;

#[allow(non_snake_case)]
pub fn GetModuleFileNameExRs<'a>(
    process: &OwnedHandle,
    module: Option<HMODULE>,
    buf: &'a mut Vec<u16>,
) -> Result<&'a U16Str> {
    let span = trace_span!(parent: CURRENT_PID.lock().clone(), "GetModuleFileNameExRs");
    let _guard = span.enter();

    let module = module.unwrap_or_default();

    let len = loop {
        let len = unsafe { GetModuleFileNameExW((**process).into(), module.into(), buf) };

        // If the size of the file name is larger than the value of the nSize parameter, the function succeeds but the
        // file name is truncated and null-terminated.
        // If the function fails, the return value is 0 (zero). To get extended error information, call GetLastError.
        if len as usize == buf.len() {
            let new_len = buf.len() + MAX_PATH as usize;

            // buffer size insufficient
            trace!(
                new_len,
                "insufficient buffer size; increasing it and trying again"
            );

            buf.resize(new_len, 0u16);

            continue;
        }

        if len == 0 {
            let err = unsafe { GetLastError() };

            error!("{err:?}");

            bail!("GetModuleFileNameExRs: {err:?}");
        }

        break len;
    };

    let path = &buf[..len as usize];

    Ok(U16Str::from_slice(path))
}
