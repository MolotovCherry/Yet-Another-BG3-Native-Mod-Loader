use std::ffi::c_void;

use shared::utils::OwnedHandle;
use tracing::error;
use windows::{
    core::Error,
    Win32::{
        Foundation::{GetLastError, HANDLE, WAIT_OBJECT_0, WIN32_ERROR},
        System::Threading::{
            CreateRemoteThread, WaitForSingleObject, INFINITE, LPTHREAD_START_ROUTINE,
        },
    },
};

pub struct RemoteThread(HANDLE);

impl RemoteThread {
    pub fn new(
        process: &OwnedHandle,
        addr: LPTHREAD_START_ROUTINE,
        lpparameter: Option<*const c_void>,
    ) -> Result<Self, Error> {
        let res = unsafe {
            CreateRemoteThread(process.as_raw_handle(), None, 0, addr, lpparameter, 0, None)
        };

        res.map(Self)
    }

    pub fn wait(&self) -> Result<(), WIN32_ERROR> {
        let res = unsafe { WaitForSingleObject(self.0, INFINITE) };
        if res == WAIT_OBJECT_0 {
            Ok(())
        } else {
            let err = unsafe { GetLastError() };
            error!(state = ?res, ?err, "object in wrong state");
            Err(err)
        }
    }
}
