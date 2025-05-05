use eyre::Result;
use shared::utils::OwnedHandle;
use windows::{
    Win32::{
        Security::{
            Authorization::{
                ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
            },
            PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
        },
        System::Threading::CreateEventW,
    },
    core::w,
};

use crate::utils::PSecurityDescriptor;

#[allow(dead_code)]
pub struct Event(OwnedHandle);

impl Event {
    pub fn new() -> Result<Self> {
        // We would like to be globally available so other apps can query the existence of yabg3nml,
        // particularly from DllMain. In DllMain you can use OpenEventW to check for existence
        //
        // https://stackoverflow.com/a/20581490/9423933
        // http://web.archive.org/web/20151215210112/http://blogs.msdn.com/b/winsdk/archive/2009/11/10/access-denied-on-a-mutex.aspx
        let sec_str = w!("D:(A;;GA;;;WD)(A;;GA;;;AN)S:(ML;;NW;;;S-1-16-0)");

        let mut psec_desc: PSecurityDescriptor = PSECURITY_DESCRIPTOR::default().into();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sec_str,
                SDDL_REVISION_1,
                psec_desc.as_mut(),
                None,
            )?;
        }

        let sec_attr = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: psec_desc.as_void(),
            bInheritHandle: false.into(),
        };

        let event = unsafe {
            CreateEventW(
                Some(&sec_attr),
                false,
                false,
                w!(r"Global\yet-another-bg3-native-mod-loader"),
            )?
        };

        let event = unsafe { OwnedHandle::new(event) };

        Ok(Self(event))
    }
}
