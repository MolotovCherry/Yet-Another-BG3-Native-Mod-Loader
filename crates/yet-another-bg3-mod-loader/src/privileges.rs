use eyre::Result;
use shared::utils::OwnedHandle;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::LUID,
        Security::{
            AdjustTokenPrivileges, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
            SE_PRIVILEGE_REMOVED, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    },
};

pub fn set_privilege(name: PCWSTR, state: bool) -> Result<()> {
    let process: OwnedHandle = {
        let process = unsafe { GetCurrentProcess() };
        process.into()
    };

    let mut token: OwnedHandle = OwnedHandle::default();
    unsafe {
        OpenProcessToken(
            process.as_raw_handle(),
            TOKEN_ADJUST_PRIVILEGES,
            token.as_mut(),
        )?;
    }

    let mut luid = LUID::default();
    unsafe {
        LookupPrivilegeValueW(PCWSTR::null(), name, &mut luid)?;
    }

    let mut tp = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        ..Default::default()
    };

    tp.Privileges[0].Luid = luid;
    tp.Privileges[0].Attributes = if state {
        SE_PRIVILEGE_ENABLED
    } else {
        SE_PRIVILEGE_REMOVED
    };

    unsafe {
        AdjustTokenPrivileges(token.as_raw_handle(), false, Some(&tp), 0, None, None)?;
    }

    Ok(())
}
