use windows::{
    core::w,
    Win32::{
        Foundation::{GetLastError, LocalFree, ERROR_ALREADY_EXISTS, HLOCAL},
        Security::{
            Authorization::{
                ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
            },
            PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
        },
        System::Threading::CreateMutexW,
    },
};

use eyre::Result;

use crate::{helpers::OwnedHandle, popup::fatal_popup};

#[allow(unused)]
pub struct SingleInstance(OwnedHandle);

impl SingleInstance {
    /// Panics and shows error popup if another instance of app already running
    /// If it succeeds, then the app will be considered free to open again once this instance drops
    pub fn new() -> Result<Self> {
        // We would like the mutex to be globally available so other apps can query the existence of yabg3ml,
        // particularly from DllMain. In DllMain you can use OpenMutexW to check for existence
        //
        // https://stackoverflow.com/a/20581490/9423933
        // http://web.archive.org/web/20151215210112/http://blogs.msdn.com/b/winsdk/archive/2009/11/10/access-denied-on-a-mutex.aspx
        let sec_str = w!("D:(A;;GA;;;WD)(A;;GA;;;AN)S:(ML;;NW;;;S-1-16-0)");

        let mut psec_desc = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sec_str,
                SDDL_REVISION_1,
                &mut psec_desc,
                None,
            )?;
        }

        let sec_attr = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: psec_desc.0,
            bInheritHandle: false.into(),
        };

        let mutex = unsafe {
            CreateMutexW(
                Some(&sec_attr),
                true,
                w!(r"Global\yet-another-bg3-mod-loader"),
            )
        };

        unsafe {
            LocalFree(HLOCAL(psec_desc.0));
        }

        let handle: OwnedHandle = match mutex {
            Ok(h) => h.into(),
            Err(e) => {
                fatal_popup("Yet Another Bg3 Mod Loader", format!("mutex failed: {e}"));
            }
        };

        match unsafe { GetLastError() } {
            e if e == ERROR_ALREADY_EXISTS => {
                fatal_popup(
                    "Yet Another Bg3 Mod Loader",
                    "Another instance is already running",
                );
            }

            e if e.is_err() => {
                fatal_popup(
                    "Yet Another Bg3 Mod Loader",
                    format!("CreateMutexW failure: {e:?}"),
                );
            }

            _ => (),
        }

        Ok(Self(handle))
    }
}
