use eyre::{bail, Result};
use tracing::{error, trace};
use widestring::U16Str;
use windows::Win32::{
    Foundation::{GetLastError, HMODULE, MAX_PATH},
    System::ProcessStatus::GetModuleFileNameExW,
};

use crate::helpers::OwnedHandle;

pub fn get_module_file_name_ex_w<'a>(
    process: &OwnedHandle,
    module: Option<HMODULE>,
    buf: &'a mut Vec<u16>,
) -> Result<&'a U16Str> {
    let module = module.unwrap_or_default();

    let len = loop {
        let len = unsafe { GetModuleFileNameExW(process.as_raw_handle(), module, buf) };

        trace!(len, buf_len = buf.len(), "GetModuleFileNameExW returned");

        // If the buffer is too small to hold the module name, the string is truncated to nSize characters including the
        // terminating null character, the function returns nSize, and the function sets the last error to ERROR_INSUFFICIENT_BUFFER.
        // If the function fails, the return value is 0 (zero). To get extended error information, call GetLastError.
        if len as usize == buf.len() {
            let new_len = buf.len() + MAX_PATH as usize;

            // buffer size insufficient
            trace!(
                new_len,
                "GetModuleFileNameExW insufficient buffer size; increasing it and trying again"
            );

            buf.resize(new_len, 0u16);

            continue;
        }

        if len == 0 {
            let err = unsafe { GetLastError() };

            error!(
                ?err,
                len,
                buf_len = buf.len(),
                "GetModuleFileNameExW error handling"
            );

            bail!("{err:?}");
        }

        break len;
    };

    let path = &buf[..len as usize];

    Ok(U16Str::from_slice(path))
}
