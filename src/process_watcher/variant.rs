use std::ops::{Deref, DerefMut};

use windows::Win32::System::Variant::{VariantClear, VariantInit, VARIANT};

pub struct Variant(VARIANT);

impl Variant {
    pub fn new() -> Self {
        Self(unsafe { VariantInit() })
    }

    pub unsafe fn as_mut_ptr(&mut self) -> *mut VARIANT {
        &mut self.0
    }
}

impl Drop for Variant {
    fn drop(&mut self) {
        unsafe {
            let _ = VariantClear(&mut self.0);
        }
    }
}

impl Deref for Variant {
    type Target = VARIANT;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Variant {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
