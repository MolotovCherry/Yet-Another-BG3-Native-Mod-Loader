use eyre::Result;
use tracing::{error, trace, warn};
use windows::Win32::{
    Foundation::{ERROR_PARTIAL_COPY, HMODULE, STILL_ACTIVE},
    System::{
        ProcessStatus::{EnumProcessModulesEx, LIST_MODULES_64BIT},
        Threading::GetExitCodeProcess,
    },
};

use crate::helpers::OwnedHandle;

/// IMPORTANT
/// Do not call CloseHandle on any of the handles returned by this function. The information comes from a
/// snapshot, so there are no resources to be freed.
pub fn enum_modules(
    process: &OwnedHandle,
    mut cb: impl FnMut(HMODULE) -> Result<bool>,
) -> Result<()> {
    let mut modules: Vec<HMODULE> = vec![HMODULE::default(); 1024];
    inner_enum_modules(process, &mut modules)?;

    for module in modules {
        if !cb(module)? {
            break;
        }
    }

    Ok(())
}

fn is_alive(process: &OwnedHandle) -> bool {
    let mut code = 0;
    let res = unsafe { GetExitCodeProcess(process.as_raw_handle(), &mut code) };

    match res {
        Ok(_) => code == STILL_ACTIVE.0 as u32,
        Err(e) => {
            error!(%e, "GetExitCodeProcess");
            false
        }
    }
}

fn inner_enum_modules(process: &OwnedHandle, modules: &mut Vec<HMODULE>) -> Result<()> {
    loop {
        let mut lpcbneeded = 0;

        let size = (modules.len() * size_of::<HMODULE>()) as u32;

        let res = unsafe {
            EnumProcessModulesEx(
                process.as_raw_handle(),
                modules.as_mut_ptr(),
                size as u32,
                &mut lpcbneeded,
                LIST_MODULES_64BIT,
            )
        };

        // To determine if the lphModule array is too small to hold all module handles for the process,
        // compare the value returned in lpcbNeeded with the value specified in cb. If lpcbNeeded is greater
        // than cb, increase the size of the array and call EnumProcessModulesEx again.
        if lpcbneeded > size {
            let n_modules = lpcbneeded as usize / size_of::<HMODULE>();
            trace!(new_len = n_modules, "resizing to len");
            modules.resize(n_modules, HMODULE::default());
            continue;
        }

        if let Err(e) = res {
            // This can be caused by:
            // - Process was terminated
            // - Missing permissions (try running as admin)
            // - Issues with disk / file(s) corrupt
            if is_alive(process) && ERROR_PARTIAL_COPY.to_hresult() == e.code() {
                // retry again because it must've been a simple error

                warn!(
                    lpcbneeded,
                    size,
                    len = modules.len(),
                    %e,
                    "EnumProcessModulesEx did partial copy, but process is still alive, retrying"
                );

                continue;
            }

            error!(%e, "EnumProcessModulesEx");

            return Err(e.into());
        }

        trace!(
            lpcbneeded,
            size,
            len = modules.len(),
            "EnumProcessModulesEx passed"
        );

        // To determine how many modules were enumerated by the call to EnumProcessModulesEx, divide the resulting
        // value in the lpcbNeeded parameter by sizeof(HMODULE).
        let n_modules = lpcbneeded as usize / size_of::<HMODULE>();
        modules.truncate(n_modules);

        break;
    }

    Ok(())
}
