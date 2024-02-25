use std::ffi::c_void;

use windows::Win32::{
    Foundation::{BOOL, HANDLE},
    System::Diagnostics::Debug::{
        IMAGE_NT_HEADERS64, MINIDUMP_CALLBACK_INFORMATION, MINIDUMP_EXCEPTION_INFORMATION,
        MINIDUMP_TYPE, MINIDUMP_USER_STREAM_INFORMATION,
    },
};
use windows_dll::dll;

#[export_name = "MiniDumpWriteDump"]
extern "system" fn mini_dump_write_dump(
    hprocess: HANDLE,
    processid: u32,
    hfile: HANDLE,
    dumptype: MINIDUMP_TYPE,
    exceptionparam: *const MINIDUMP_EXCEPTION_INFORMATION,
    userstreamparam: *const MINIDUMP_USER_STREAM_INFORMATION,
    callbackparam: *const MINIDUMP_CALLBACK_INFORMATION,
) -> BOOL {
    #[dll(Dbghelp)]
    extern "system" {
        #[allow(non_snake_case)]
        fn MiniDumpWriteDump(
            hprocess: HANDLE,
            processid: u32,
            hfile: HANDLE,
            dumptype: MINIDUMP_TYPE,
            exceptionparam: *const MINIDUMP_EXCEPTION_INFORMATION,
            userstreamparam: *const MINIDUMP_USER_STREAM_INFORMATION,
            callbackparam: *const MINIDUMP_CALLBACK_INFORMATION,
        ) -> BOOL;
    }

    unsafe {
        MiniDumpWriteDump(
            hprocess,
            processid,
            hfile,
            dumptype,
            exceptionparam,
            userstreamparam,
            callbackparam,
        )
    }
}

#[export_name = "ImageNtHeader"]
extern "system" fn image_nt_header(base: *const c_void) -> *mut IMAGE_NT_HEADERS64 {
    #[dll(Dbghelp)]
    extern "system" {
        #[allow(non_snake_case)]
        fn ImageNtHeader(base: *const c_void) -> *mut IMAGE_NT_HEADERS64;
    }

    unsafe { ImageNtHeader(base) }
}
