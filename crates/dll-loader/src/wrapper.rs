use std::ffi::c_void;

use windows::{
    core::{GUID, HRESULT},
    Win32::Graphics::DirectWrite::DWRITE_FACTORY_TYPE,
};
use windows_targets::link;

// ordinarily I'd use windows-sys, but this function seems to be missing from it
// while it's present in windows, it has too much magic to be useful
link!("dwrite.dll" "system" fn DWriteCreateFactory(factorytype: DWRITE_FACTORY_TYPE, iid: *const GUID, factory: *mut *mut c_void) -> HRESULT);

#[export_name = "DWriteCreateFactory"]
extern "system" fn d_write_create_factory(
    factorytype: DWRITE_FACTORY_TYPE,
    iid: *const GUID,
    factory: *mut *mut c_void,
) -> HRESULT {
    unsafe { DWriteCreateFactory(factorytype, iid, factory) }
}
