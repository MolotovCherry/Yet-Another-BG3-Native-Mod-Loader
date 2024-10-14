use eyre::{bail, Result};
use shared::utils::OwnedHandle;
use tracing::{error, trace, trace_span};
use widestring::U16Str;
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{ERROR_INSUFFICIENT_BUFFER, MAX_PATH},
        System::Threading::{QueryFullProcessImageNameW, PROCESS_NAME_WIN32},
    },
};

pub fn query_full_process_image_name_w<'a>(
    process: &OwnedHandle,
    buf: &'a mut Vec<u16>,
) -> Result<&'a U16Str> {
    let span = trace_span!("query_full_process_image_name_w");
    let _guard = span.enter();

    loop {
        let mut size = buf.len() as u32;

        let res = unsafe {
            QueryFullProcessImageNameW(
                process.as_raw_handle(),
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut size,
            )
        };

        if let Err(e) = res {
            if e.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult() {
                let new_len = buf.len() + MAX_PATH as usize;

                // buffer size insufficient
                trace!(
                    new_len,
                    "insufficient buffer size; increasing it and trying again"
                );

                buf.resize(new_len, 0u16);

                continue;
            }

            error!(err = %e);
            bail!("QueryFullProcessImageNameW {e}");
        }

        let slice = {
            let s = &buf[..size as usize];
            U16Str::from_slice(s)
        };

        return Ok(slice);
    }
}
