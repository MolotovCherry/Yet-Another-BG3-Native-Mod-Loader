use eyre::Result;
use shared::utils::OwnedHandle;
use windows::{
    Win32::{
        Foundation::LUID,
        Security::{
            AdjustTokenPrivileges, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
            SE_PRIVILEGE_REMOVED, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    },
    core::PCWSTR,
};

pub fn set_privilege(name: PCWSTR, state: bool) -> Result<()> {
    let process = unsafe { GetCurrentProcess() };

    let mut token: OwnedHandle = OwnedHandle::default();
    unsafe {
        OpenProcessToken(process, TOKEN_ADJUST_PRIVILEGES, &mut *token)?;
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
        AdjustTokenPrivileges(*token, false, Some(&tp), 0, None, None)?;
    }

    Ok(())
}
