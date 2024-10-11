use eyre::Error;
use windows::Win32::{
    Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

use crate::helpers::{tri, OwnedHandle};

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

        let mut elevation = TOKEN_ELEVATION::default();
        unsafe {
            GetTokenInformation(
                token.as_raw_handle(),
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                size_of::<TOKEN_ELEVATION>() as _,
                &mut 0,
            )?;
        }

        let elevated = elevation.TokenIsElevated > 0;
        Ok::<_, Error>(elevated)
    };

    res.unwrap_or(false)
}
