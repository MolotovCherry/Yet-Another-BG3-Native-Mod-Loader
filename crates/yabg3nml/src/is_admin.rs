use eyre::Error;
use shared::utils::{tri, OwnedHandle};
use windows::Win32::{
    Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

pub fn is_admin() -> bool {
    let res = tri! {
        let process: OwnedHandle = {
            let process = unsafe { GetCurrentProcess() };
            process.into()
        };

        let mut token: OwnedHandle = OwnedHandle::default();
        unsafe {
            OpenProcessToken(process.as_raw_handle(), TOKEN_QUERY, token.as_mut())?;
        }

        let mut _written = 0;
        let mut elevation = TOKEN_ELEVATION::default();
        unsafe {
            GetTokenInformation(
                token.as_raw_handle(),
                TokenElevation,
                Some(&raw mut elevation as *mut _),
                size_of::<TOKEN_ELEVATION>() as _,
                &mut _written,
            )?;
        }

        let elevated = elevation.TokenIsElevated > 0;
        Ok::<_, Error>(elevated)
    };

    res.unwrap_or(false)
}
