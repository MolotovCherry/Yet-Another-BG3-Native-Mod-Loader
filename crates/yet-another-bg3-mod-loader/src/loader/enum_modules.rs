use eyre::Result;
use tracing::{error, trace};
use windows::{
    core::HRESULT,
    Win32::{
        Foundation::{HMODULE, STILL_ACTIVE},
        System::{
            ProcessStatus::{EnumProcessModulesEx, LIST_MODULES_64BIT},
            Threading::GetExitCodeProcess,
        },
    },
};

use crate::helpers::OwnedHandle;

pub fn enum_modules(
    process: &OwnedHandle,
    mut cb: impl FnMut(HMODULE) -> Result<bool>,
) -> Result<()> {
    fn inner(process: &OwnedHandle, modules: &mut Vec<HMODULE>) -> Result<()> {
        // fully initialize this in order to set len
        let mut lpcbneeded = 0;

        loop {
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

            if let Err(e) = res {
                const ERROR_PARTIAL_COPY: HRESULT =
                    windows::Win32::Foundation::ERROR_PARTIAL_COPY.to_hresult();

                // This can be caused by:
                // - Process was terminated
                // - Missing permissions (try running as admin)
                // - Issues with disk / file(s) corrupt
                const ERROR_PARTIAL_COPY_2: HRESULT = HRESULT::from_win32(0x8007012B);

                trace!(?e, "EnumProcessModulesEx partial copy err");

                if is_alive(process)
                    && [ERROR_PARTIAL_COPY, ERROR_PARTIAL_COPY_2].contains(&e.code())
                {
                    // retry again because it must've been a simple error

                    trace!(
                        lpcbneeded,
                        size,
                        len = modules.len(),
                        "process is still alive"
                    );

                    // To determine if the lphModule array is too small to hold all module handles for the process,
                    // compare the value returned in lpcbNeeded with the value specified in cb. If lpcbNeeded is greater
                    // than cb, increase the size of the array and call EnumProcessModulesEx again.
                    if lpcbneeded > size {
                        let n_modules = lpcbneeded as usize / size_of::<HMODULE>();
                        trace!(new_len = n_modules, "resizing to len");
                        modules.resize(n_modules, HMODULE::default());
                    }

                    continue;
                }

                error!(?e, "EnumProcessModulesEx received err");

                return Err(e.into());
            }

            trace!(
                lpcbneeded,
                size,
                len = modules.len(),
                "EnumProcessModulesEx passed"
            );

            // To determine if the lphModule array is too small to hold all module handles for the process,
            // compare the value returned in lpcbNeeded with the value specified in cb. If lpcbNeeded is greater
            // than cb, increase the size of the array and call EnumProcessModulesEx again.
            if lpcbneeded > size {
                let n_modules = lpcbneeded as usize / size_of::<HMODULE>();
                trace!(new_len = n_modules, "resizing to len");
                modules.resize(n_modules, HMODULE::default());
                continue;
            }

            // IMPORTANT
            // Do not call CloseHandle on any of the handles returned by this function. The information comes from a
            // snapshot, so there are no resources to be freed.

            break;
        }

        // To determine how many modules were enumerated by the call to EnumProcessModulesEx, divide the resulting
        // value in the lpcbNeeded parameter by sizeof(HMODULE).
        let n_modules = lpcbneeded as usize / size_of::<HMODULE>();
        modules.truncate(n_modules);

        Ok(())
    }

    let mut modules: Vec<HMODULE> = vec![HMODULE::default(); 1024];
    inner(process, &mut modules)?;

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
            error!("GetExitCodeProcess: {e}");
            false
        }
    }
}
