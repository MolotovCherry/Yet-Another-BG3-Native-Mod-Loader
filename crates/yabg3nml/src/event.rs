use eyre::Result;
use shared::utils::OwnedHandle;
use windows::{
    core::w,
    Win32::{
        Foundation::{LocalFree, HLOCAL},
        Security::{
            Authorization::{
                ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
            },
            PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
        },
        System::Threading::CreateEventW,
    },
};

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

        let event: OwnedHandle = unsafe {
            CreateEventW(
                Some(&sec_attr),
                false,
                false,
                w!(r"Global\yet-another-bg3-native-mod-loader"),
            )?
            .into()
        };

        unsafe {
            LocalFree(HLOCAL(psec_desc.0));
        }

        Ok(Self(event))
    }
}
