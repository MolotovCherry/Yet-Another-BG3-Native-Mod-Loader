use windows::{Win32::Foundation::HANDLE, core::Owned};

pub type OwnedHandle = Owned<HANDLE>;

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
