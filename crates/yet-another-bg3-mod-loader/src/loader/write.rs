use std::{ffi::c_void, sync::LazyLock};

use eyre::{bail, Result};
use tracing::{error, trace};
use windows::Win32::{
    Foundation::GetLastError,
    System::{
        Diagnostics::Debug::WriteProcessMemory,
        Memory::{
            MemExtendedParameterAddressRequirements, VirtualAlloc2, MEM_ADDRESS_REQUIREMENTS,
            MEM_COMMIT, MEM_EXTENDED_PARAMETER, MEM_RESERVE, PAGE_READWRITE,
        },
        SystemInformation::{GetSystemInfo, SYSTEM_INFO},
    },
};

use crate::{helpers::OwnedHandle, popup::warn_popup};

struct SystemInfo(SYSTEM_INFO);
unsafe impl Send for SystemInfo {}
unsafe impl Sync for SystemInfo {}

/// # Safety
/// process must have PROCESS_VM_OPERATION access right.
/// data must be valid ptr for size reads
/// size must be a multiple of size_of::<T>()
pub unsafe fn write_in<T>(
    process: &OwnedHandle,
    data: *const T,
    size: usize,
) -> Result<*const c_void> {
    static INFO: LazyLock<SystemInfo> = LazyLock::new(|| {
        let mut info = SYSTEM_INFO::default();
        unsafe {
            GetSystemInfo(&mut info);
        }

        SystemInfo(info)
    });

    // > the Alignment must be 2 ^ n where n >= 0x10. so >= 0x10000. or Alignment can be 0. the 1024 (0x400) is invalid value for aligment,
    // > despite it 2^0xa - the 0xa too small for aligment
    // https://stackoverflow.com/questions/54223343/virtualalloc2-with-memextendedparameteraddressrequirements-always-produces-error
    let align = align_of::<T>().max(INFO.0.dwAllocationGranularity as usize);
    let mut requirements = MEM_ADDRESS_REQUIREMENTS {
        LowestStartingAddress: INFO.0.lpMinimumApplicationAddress,
        // The ending address must align to page boundary - 1
        // https://stackoverflow.com/questions/54223343/virtualalloc2-with-memextendedparameteraddressrequirements-always-produces-error
        HighestEndingAddress: INFO.0.lpMaximumApplicationAddress,
        Alignment: align,
    };

    trace!(?requirements, "VirtualAlloc2 alloc reqs");

    debug_assert!(align % align_of::<T>() == 0);

    let mut param = MEM_EXTENDED_PARAMETER::default();
    param.Anonymous1._bitfield = MemExtendedParameterAddressRequirements.0 as u64;
    param.Anonymous2.Pointer = (&mut requirements as *mut MEM_ADDRESS_REQUIREMENTS).cast();

    // The size must always be a multiple of the page size.
    let alloc_size = size.next_multiple_of(INFO.0.dwPageSize as usize);

    trace!(%size, %alloc_size, "VirtualAlloc2 size");

    let list = &mut [param];
    let alloc_addr = {
        let addr = unsafe {
            VirtualAlloc2(
                process.as_raw_handle(),
                None,
                alloc_size,
                MEM_RESERVE | MEM_COMMIT,
                PAGE_READWRITE.0,
                Some(list),
            )
        };

        if addr.is_null() {
            let error = unsafe { GetLastError() };
            let error = error.to_hresult();
            error!(%error, "VirtualAlloc2 failed to allocate");

            warn_popup(
                "Allocation failure",
                format!("Failed to allocate in target process\n\nThis could be due to multiple reasons, but in any case, winapi returned an error that we cannot recover from. Recommend restarting game and trying again\n\nError: {error}"),
            );

            bail!("failed to allocate");
        }

        addr
    };

    // Write the data to the process
    let res =
        unsafe { WriteProcessMemory(process.as_raw_handle(), alloc_addr, data.cast(), size, None) };

    if let Err(e) = res {
        error!(?e, "Failed to write to process");

        warn_popup(
            "Write failure",
            format!("Failed to write to process memory\n\nThis could be due to multiple reasons, but in any case, winapi returned an error that we cannot recover from. Recommend restarting game and trying again\n\nError: {e}"),
        );

        bail!("failed to write to process");
    }

    Ok(alloc_addr)
}
