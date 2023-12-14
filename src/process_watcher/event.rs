use windows::Win32::{
    Foundation::HANDLE,
    System::Threading::{CreateEventW, SetEvent},
};

use crate::helpers::OwnedHandle;

#[derive(Debug)]
pub struct Event(Option<OwnedHandle>);

impl Event {
    pub fn new() -> windows::core::Result<Self> {
        let event: OwnedHandle = unsafe { CreateEventW(None, false, false, None)?.into() };

        Ok(Self(Some(event)))
    }

    pub fn signal(&mut self) -> windows::core::Result<()> {
        if let Some(handle) = self.0.take() {
            unsafe {
                SetEvent(handle.as_raw_handle())?;
            }
        }

        Ok(())
    }

    pub fn get(&self) -> Option<HANDLE> {
        self.0.as_ref().map(|r| r.as_raw_handle())
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            unsafe {
                let _ = SetEvent(handle.as_raw_handle());
            }
        }
    }
}
