use eyre::Result;
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{FreeLibrary, HINSTANCE, HMODULE},
        System::LibraryLoader::{
            FreeLibraryAndExitThread, GetModuleHandleExA, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
        },
    },
};

/// Container for a loaded plugin. Frees itself on drop
#[derive(Default)]
pub struct Plugin(pub HMODULE);

unsafe impl Send for Plugin {}

impl Drop for Plugin {
    fn drop(&mut self) {
        _ = unsafe { FreeLibrary(self.0) };
    }
}

/// Wrapper for HINSTANCE being thread safe
#[derive(Copy, Clone, Default)]
pub struct HInstance(pub HINSTANCE);
unsafe impl Send for HInstance {}
unsafe impl Sync for HInstance {}

/// This increfs self dll, decrefs on drop
/// This ensures that while we hold the guard, our library cannot be freed
///
/// Warning, instantly shuts down thread after exit to avoid last decref
/// causing code to disappear. This means things like handle.join() on a
/// thread will break rust. NO CODE after the drop of this is executed
///
/// https://devblogs.microsoft.com/oldnewthing/20131105-00/?p=2733
pub struct FreeSelfLibrary(HMODULE);
unsafe impl Send for FreeSelfLibrary {}
unsafe impl Sync for FreeSelfLibrary {}

impl FreeSelfLibrary {
    pub fn new(inst: HINSTANCE) -> Result<Self> {
        let mut hmod = HMODULE::default();
        unsafe {
            GetModuleHandleExA(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                PCSTR(inst.0.cast()),
                &mut hmod,
            )?;
        }

        Ok(Self(hmod))
    }
}

impl Drop for FreeSelfLibrary {
    fn drop(&mut self) {
        unsafe { FreeLibraryAndExitThread(self.0, 0) };
    }
}
