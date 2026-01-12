use std::ffi::c_void;

use eyre::{Result, bail};
use shared::{popup::warn_popup, utils::OwnedHandle};
use tracing::{error, trace_span};
use windows::Win32::{
    Foundation::GetLastError,
    System::{
        Diagnostics::Debug::WriteProcessMemory,
        Memory::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, VirtualAllocEx},
    },
};

pub fn write_in<T>(process: &OwnedHandle, data: *const T, size: usize) -> Result<*const c_void> {
    let span = trace_span!("write_in");
    let _guard = span.enter();

    let alloc_addr = {
        let addr = unsafe {
            VirtualAllocEx(
                **process,
                None,
                size,
                MEM_RESERVE | MEM_COMMIT,
                PAGE_READWRITE,
            )
        };

        if addr.is_null() {
            let error = {
                let e = unsafe { GetLastError() };
                e.to_hresult()
            };

            error!(%error, "VirtualAllocEx failed to allocate memory");

            warn_popup(
                "Allocation failure",
                format!(
                    "Failed to allocate in target process. Patching has been aborted on this process.\n\nThis could be due to multiple reasons, but in any case, winapi returned an error. Recommend restarting game and trying again. Press OK to continue; this tool will continue to operate normally.\n\nError: {error}"
                ),
            );

            bail!("{error}");
        }

        addr
    };

    assert!(
        (alloc_addr as usize).is_multiple_of(align_of::<T>()),
        "alloc @ {alloc_addr:?} has insufficient alignment for align {}",
        align_of::<T>()
    );

    // Write the data to the process
    let res = unsafe { WriteProcessMemory(**process, alloc_addr, data.cast(), size, None) };

    if let Err(e) = res {
        error!(?e, "Failed to write to process");

        warn_popup(
            "Write failure",
            format!(
                "Failed to write to process memory. Patching has been aborted on this process.\n\nThis could be due to multiple reasons, but in any case, winapi returned an error. Recommend restarting game and trying again. Press OK to continue; this tool will continue to operate normally.\n\nError: {e}"
            ),
        );

        bail!("{e}");
    }

    Ok(alloc_addr)
}
