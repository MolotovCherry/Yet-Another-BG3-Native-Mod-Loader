use eyre::Error;
use shared::utils::{OwnedHandle, tri};
use tracing::trace;
use windows::Win32::{
    Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation},
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

pub fn is_admin() -> bool {
    let res = tri! {
        let process = unsafe { GetCurrentProcess() };

        let mut token = OwnedHandle::default();
        unsafe {
            OpenProcessToken(process, TOKEN_QUERY, &mut *token)?;
        }

        let mut elevation = TOKEN_ELEVATION::default();
        unsafe {
            GetTokenInformation(
                *token,
                TokenElevation,
                Some(&raw mut elevation as *mut _),
                size_of::<TOKEN_ELEVATION>() as _,
                &mut 0,
            )?;
        }

        let elevated = elevation.TokenIsElevated > 0;
        Ok::<_, Error>(elevated)
    };

    let admin = res.unwrap_or(false);

    trace!(
        "we are {}running as admin.{}",
        if !admin { "not " } else { "" },
        if !admin {
            " with watcher/injector, it's possible the bg3 process may get missed. if this happens, run this tool as admin"
        } else {
            ""
        }
    );

    admin
}
