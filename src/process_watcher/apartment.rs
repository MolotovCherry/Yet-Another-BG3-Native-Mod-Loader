use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT};

pub struct Apartment;

impl Apartment {
    pub fn new(flags: COINIT) -> windows::core::Result<Self> {
        unsafe {
            CoInitializeEx(None, flags)?;
        }

        Ok(Self)
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
