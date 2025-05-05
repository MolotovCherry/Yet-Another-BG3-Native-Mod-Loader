use eyre::Error;
use shared::utils::{OwnedHandle, tri};
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

    res.unwrap_or(false)
}
