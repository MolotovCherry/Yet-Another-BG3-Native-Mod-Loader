use std::thread::{self, JoinHandle};

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

pub struct ThreadManager(Option<Vec<JoinHandle<()>>>);

impl ThreadManager {
    pub fn new() -> Self {
        Self(Some(Vec::new()))
    }

    pub fn spawn<F>(&mut self, f: F)
    where
        F: FnOnce(),
        F: Send + 'static,
    {
        let handle = thread::spawn(f);
        self.0.as_mut().unwrap().push(handle);
    }
}

impl Drop for ThreadManager {
    fn drop(&mut self) {
        let threads = self.0.take().unwrap();
        for thread in threads {
            _ = thread.join();
        }
    }
}
