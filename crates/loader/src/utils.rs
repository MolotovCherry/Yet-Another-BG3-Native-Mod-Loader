use windows::Win32::Foundation::{FreeLibrary, HMODULE};

/// Container for a loaded plugin. Frees itself on drop
#[derive(Default)]
pub struct Plugin(pub HMODULE);

unsafe impl Send for Plugin {}

impl Drop for Plugin {
    fn drop(&mut self) {
        _ = unsafe { FreeLibrary(self.0) };
    }
}
