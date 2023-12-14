use std::sync::Mutex;

use windows::Win32::{
    Foundation::HANDLE,
    System::Threading::{CreateEventW, SetEvent},
};

use crate::helpers::OwnedHandle;

#[derive(Debug)]
pub struct Event(Mutex<Option<OwnedHandle>>);

impl Event {
    pub fn new() -> windows::core::Result<Self> {
        let event: OwnedHandle = unsafe { CreateEventW(None, false, false, None)?.into() };

        Ok(Self(Mutex::new(Some(event))))
    }

    pub fn signal(&self) -> windows::core::Result<()> {
        let mut lock = self.0.lock().unwrap();

        if let Some(handle) = lock.take() {
            unsafe {
                SetEvent(handle.as_raw_handle())?;
            }
        }

        Ok(())
    }

    pub fn get(&self) -> Option<HANDLE> {
        self.0.lock().unwrap().as_ref().map(|r| r.as_raw_handle())
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        _ = self.signal();
    }
}
