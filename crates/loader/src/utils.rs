use std::{
    mem,
    ops::Deref,
    thread::{self, JoinHandle},
};

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

pub struct ThreadedWrapper<T>(T);
unsafe impl<T> Send for ThreadedWrapper<T> {}
unsafe impl<T> Sync for ThreadedWrapper<T> {}

impl<T> ThreadedWrapper<T> {
    /// # Safety
    /// Caller asserts that T is safe to use in Send+Sync contexts
    pub unsafe fn new(t: T) -> Self {
        Self(t)
    }
}

impl<T> Deref for ThreadedWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
