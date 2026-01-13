use std::ops::{Deref, DerefMut};

use windows::{Win32::Foundation::HANDLE, core::Owned};

pub type OwnedHandle = Owned<HANDLE>;

#[repr(transparent)]
pub struct ThreadedWrapper<T>(T);
unsafe impl<T> Send for ThreadedWrapper<T> {}
unsafe impl<T> Sync for ThreadedWrapper<T> {}

impl<T: Default> Default for ThreadedWrapper<T> {
    fn default() -> Self {
        Self(T::default())
    }
}

impl<T> ThreadedWrapper<T> {
    /// # Safety
    /// Caller asserts that T is safe to use in Send+Sync contexts
    pub unsafe fn new(t: T) -> Self {
        Self(t)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for ThreadedWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ThreadedWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Poor mans try {} blocks
#[macro_export]
macro_rules! tri {
    ($($code:tt)*) => {
        (|| {
            $(
                $code
            )*
        })()
    };
}
pub use tri;
