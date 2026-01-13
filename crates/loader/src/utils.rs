use std::{
    mem,
    thread::{self, JoinHandle},
};

use windows::{Win32::Foundation::HMODULE, core::Free};

/// Container for a loaded plugin. Frees itself on drop
#[derive(Default)]
pub struct Plugin(pub HMODULE);
unsafe impl Send for Plugin {}

impl Drop for Plugin {
    fn drop(&mut self) {
        unsafe {
            self.0.free();
        }
    }
}

pub struct ThreadManager(Vec<JoinHandle<()>>);

impl ThreadManager {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn spawn<F>(&mut self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let handle = thread::spawn(f);
        self.0.push(handle);
    }
}

impl Drop for ThreadManager {
    fn drop(&mut self) {
        let threads = mem::take(&mut self.0);
        for thread in threads {
            _ = thread.join();
        }
    }
}
