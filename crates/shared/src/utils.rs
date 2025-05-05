use std::sync::{Mutex, MutexGuard};

use windows::{Win32::Foundation::HANDLE, core::Owned};

pub type OwnedHandle = Owned<HANDLE>;

pub trait SuperLock<T> {
    fn super_lock(&self) -> MutexGuard<T>;
}

impl<T> SuperLock<T> for Mutex<T> {
    /// Always get a mutex guard regardless of poison status
    fn super_lock(&self) -> MutexGuard<T> {
        self.clear_poison();

        match self.lock() {
            Ok(v) => v,
            Err(_) => unreachable!("poison was cleared; this is impossible"),
        }
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
