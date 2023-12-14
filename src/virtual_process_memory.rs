use std::ffi::c_void;

use windows::Win32::{
    Foundation::HANDLE,
    System::Memory::{VirtualFreeEx, MEM_RELEASE},
};

#[derive(Debug)]
pub struct VirtualProcessMemory {
    process: HANDLE,
    memory: *mut c_void,
}

impl VirtualProcessMemory {
    pub fn new(process: HANDLE, memory: *mut c_void) -> windows::core::Result<Self> {
        if memory.is_null() {
            Err(windows::core::Error::from_win32())
        } else {
            Ok(Self { process, memory })
        }
    }

    pub fn get(&self) -> *mut c_void {
        self.memory
    }
}

impl Drop for VirtualProcessMemory {
    fn drop(&mut self) {
        unsafe {
            let _ = VirtualFreeEx(self.process, self.memory, 0, MEM_RELEASE);
        }
    }
}
