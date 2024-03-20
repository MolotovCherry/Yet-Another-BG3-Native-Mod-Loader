// Thanks to @ https://github.com/sidit77/client-silencer @ for this useful code

use std::{
    ffi::{c_void, CStr},
    mem::size_of,
    ptr::NonNull,
};
use std::{ptr::addr_of, sync::Mutex};

use eyre::{bail, Result};
use windows::Win32::System::Diagnostics::Debug::*;
use windows::Win32::System::SystemServices::*;
use windows::Win32::System::WindowsProgramming::*;
use windows::Win32::System::{
    LibraryLoader::GetModuleHandleW,
    Memory::{VirtualProtect, PAGE_PROTECTION_FLAGS, PAGE_READWRITE},
};

use crate::ensure;
use crate::utils::{IntPtr, IterPtr, RawIterPtr};

macro_rules! ensure {
    ($cond:expr, $result:expr) => {
        if !($cond) {
            return Err($result);
        }
    };
}

macro_rules! impl_fn {
    () => {
        impl<Result> FnUtils for extern "C" fn() -> Result {
            fn as_usize(&self) -> usize {
                *self as usize
            }
        }
    };

    ($($arg:tt),*) => {
        impl<$($arg),*, Result> FnUtils for extern "C" fn($($arg),*) -> Result {
            fn as_usize(&self) -> usize {
                *self as usize
            }
        }
    };
}

#[cfg(target_pointer_width = "32")]
#[allow(non_camel_case_types)]
type IMAGE_NT_HEADERS = IMAGE_NT_HEADERS32;
#[cfg(target_pointer_width = "64")]
#[allow(non_camel_case_types)]
type IMAGE_NT_HEADERS = IMAGE_NT_HEADERS64;

#[cfg(target_pointer_width = "32")]
#[allow(non_camel_case_types)]
type IMAGE_THUNK_DATA = IMAGE_THUNK_DATA32;
#[cfg(target_pointer_width = "64")]
#[allow(non_camel_case_types)]
type IMAGE_THUNK_DATA = IMAGE_THUNK_DATA64;

#[cfg(target_pointer_width = "32")]
const IMAGE_ORDINAL_FLAG: u32 = IMAGE_ORDINAL_FLAG32;
#[cfg(target_pointer_width = "64")]
const IMAGE_ORDINAL_FLAG: u64 = IMAGE_ORDINAL_FLAG64;

pub trait FnUtils {
    fn as_usize(&self) -> usize;
}

impl_fn!();
impl_fn!(A);
impl_fn!(A, B);
impl_fn!(A, B, C);
impl_fn!(A, B, C, D);
impl_fn!(A, B, C, D, E);
impl_fn!(A, B, C, D, E, F);
impl_fn!(A, B, C, D, E, F, G);
impl_fn!(A, B, C, D, E, F, G, H);
impl_fn!(A, B, C, D, E, F, G, H, I);
impl_fn!(A, B, C, D, E, F, G, H, I, J);
impl_fn!(A, B, C, D, E, F, G, H, I, J, K);
impl_fn!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
impl_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
impl_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
impl_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
impl_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
impl_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U);
impl_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V);
impl_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W);
impl_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X);
impl_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y);
impl_fn!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);

pub struct IATHook<F> {
    callable: usize,
    module: String,
    func_name: String,
    inner: Mutex<Inner<F>>,
}

struct Inner<F> {
    iat_entry: Option<NonNull<usize>>,
    old_fn: Option<NonNull<F>>,
    hooked: bool,
}

unsafe impl<F> Send for IATHook<F> {}
unsafe impl<F> Sync for IATHook<F> {}

#[allow(private_bounds)]
impl<F: Copy + FnUtils> IATHook<F> {
    /// SAFETY: Caller guarantees callable has the right signature
    pub unsafe fn new(module: &str, func_name: &str, callable: impl Copy + FnUtils) -> Self {
        let inner = Inner {
            iat_entry: None,
            old_fn: None,
            hooked: false,
        };

        Self {
            inner: Mutex::new(inner),
            callable: callable.as_usize(),
            module: module.to_owned(),
            func_name: func_name.to_owned(),
        }
    }

    /// Gets old function. Panics if old function does not exist (if you haven't called install yet)
    pub fn call(&self) -> F {
        let guard = self.inner.lock().unwrap();
        let ptr = guard.old_fn.unwrap();

        unsafe { ptr.as_ptr().read() }
    }

    // Returns whether unhook succeeded or not
    pub fn uninstall(&self) -> bool {
        let mut guard = self.inner.lock().unwrap();

        if guard.hooked {
            let Some(iat_entry) = guard.iat_entry else {
                return false;
            };

            let Some(old_fn) = guard.old_fn else {
                return false;
            };

            let unhooked = unsafe { write_protected(iat_entry.as_ptr().cast(), old_fn).is_ok() };

            guard.hooked = !unhooked;

            return unhooked;
        }

        false
    }

    pub fn install(&self) -> Result<()> {
        let mut guard = self.inner.lock().unwrap();

        if guard.hooked {
            bail!("Already hooked");
        }

        // check if we already did the work to find the address
        if let Some(iat_entry) = guard.iat_entry {
            let hooked = unsafe { write_protected(iat_entry.as_ptr().cast(), self.callable) };

            guard.hooked = hooked.is_ok();
            hooked?;

            return Ok(());
        }

        let base = unsafe { GetModuleHandleW(None)?.0 as u64 };

        let mut size = 0;
        let import_desc = unsafe {
            ImageDirectoryEntryToData(
                base as *const _,
                true,
                IMAGE_DIRECTORY_ENTRY_IMPORT,
                &mut size,
            )
        };

        if import_desc.is_null() {}

        todo!()
    }
}

pub unsafe fn write_protected<T>(src: *const c_void, data: T) -> Result<()> {
    let mut protection = PAGE_PROTECTION_FLAGS(0);
    unsafe {
        VirtualProtect(src, size_of::<T>(), PAGE_READWRITE, &mut protection)?;
    }

    let target = src as *mut T;

    unsafe {
        target.write(data);
    }

    unsafe {
        VirtualProtect(src, size_of::<T>(), protection, &mut protection)?;
    }

    Ok(())
}
