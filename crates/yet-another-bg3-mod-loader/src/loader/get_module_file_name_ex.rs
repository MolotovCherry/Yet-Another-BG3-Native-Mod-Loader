use eyre::{bail, Result};
use tracing::{error, trace};
use widestring::U16Str;
use windows::Win32::{
    Foundation::{GetLastError, ERROR_INSUFFICIENT_BUFFER, HMODULE},
    System::ProcessStatus::GetModuleFileNameExW,
};

use crate::helpers::OwnedHandle;

pub fn get_module_file_name_ex_w<'a>(
    process: &OwnedHandle,
    module: HMODULE,
    buf: &'a mut Vec<u16>,
) -> Result<&'a U16Str> {
    let len = loop {
        let len = unsafe { GetModuleFileNameExW(process.as_raw_handle(), module, buf) };

        // If the buffer is too small to hold the module name, the string is truncated to nSize characters including the
        // terminating null character, the function returns nSize, and the function sets the last error to ERROR_INSUFFICIENT_BUFFER.
        // If the function fails, the return value is 0 (zero). To get extended error information, call GetLastError.
        if len as usize == buf.len() || len == 0 {
            let err = unsafe { GetLastError() };

            if err.is_ok() {
                break len;
            }

            if err == ERROR_INSUFFICIENT_BUFFER {
                trace!("ERROR_INSUFFICIENT_BUFFER, increasing +1024");
                buf.resize(buf.len() + 1024, 0u16);
                continue;
            }

            if len == 0 {
                error!(?err, "GetModuleBaseNameW returned 0");
                bail!("{err:?}");
            }
        }

        break len;
    };

    let path = &buf[..len as usize];

    Ok(U16Str::from_slice(path))
}
