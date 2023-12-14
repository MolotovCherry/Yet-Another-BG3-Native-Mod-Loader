use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Mutex},
    thread::{self, JoinHandle},
};

use anyhow::Context;

use windows::{
    core::{ComInterface, BSTR},
    Win32::{
        Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::{
            Com::{
                CoCreateInstance, CoInitializeSecurity, CoSetProxyBlanket, CLSCTX_INPROC_SERVER,
                CLSCTX_LOCAL_SERVER, COINIT_DISABLE_OLE1DDE, COINIT_MULTITHREADED, EOAC_NONE,
                RPC_C_AUTHN_LEVEL_CALL, RPC_C_AUTHN_LEVEL_DEFAULT, RPC_C_IMP_LEVEL_IMPERSONATE,
            },
            Rpc::{RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE},
            Threading::{WaitForSingleObject, INFINITE},
            Wmi::{
                IUnsecuredApartment, IWbemLocator, IWbemObjectSink, UnsecuredApartment,
                WbemLocator, WBEM_FLAG_SEND_STATUS,
            },
        },
    },
};

use super::{apartment::Apartment, event::Event, event_sink::EventSink};

#[derive(Debug)]
pub enum CallType {
    Pid(u32),
    Timeout,
}

pub struct ProcessWatcher {
    event: Event,
    thread: Mutex<Option<JoinHandle<anyhow::Result<()>>>>,
}

impl ProcessWatcher {
    pub fn wait(&self) -> anyhow::Result<()> {
        self.thread.lock().unwrap().take().unwrap().join().unwrap()
    }

    pub fn stop(&self) {
        _ = self.event.signal();
    }

    pub fn watch(
        processes: Vec<PathBuf>,
        cb: impl Fn(CallType) + Send + Clone + 'static,
    ) -> anyhow::Result<Self> {
        Self::watch_timeout(processes, INFINITE, cb)
    }

    pub fn watch_timeout(
        processes: Vec<PathBuf>,
        timeout_ms: u32,
        cb: impl Fn(CallType) + Send + Clone + 'static,
    ) -> anyhow::Result<Self> {
        let event = Event::new()?;
        let event_handle = event.get().unwrap();

        let thread = thread::spawn(move || {
            let _apartment = Apartment::new(COINIT_MULTITHREADED | COINIT_DISABLE_OLE1DDE)?;

            unsafe {
                CoInitializeSecurity(
                    None,
                    -1,
                    None,
                    None,
                    RPC_C_AUTHN_LEVEL_DEFAULT,
                    RPC_C_IMP_LEVEL_IMPERSONATE,
                    None,
                    EOAC_NONE,
                    None,
                )
                .context("Failed to CoInitializeSecurity")?;
            }

            let locator: IWbemLocator = unsafe {
                CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER)
                    .context("Failed to CoCreateInstance for WbemLocator")?
            };
            let services = unsafe {
                locator
                    .ConnectServer(&BSTR::from("ROOT\\CIMV2"), None, None, None, 0, None, None)
                    .context("Failed to ConnectServer")?
            };

            unsafe {
                CoSetProxyBlanket(
                    &services,
                    RPC_C_AUTHN_WINNT,
                    RPC_C_AUTHZ_NONE,
                    None,
                    RPC_C_AUTHN_LEVEL_CALL,
                    RPC_C_IMP_LEVEL_IMPERSONATE,
                    None,
                    EOAC_NONE,
                )
                .context("Failed to CoSetProxyBlanket")?;
            }

            let unsecured_apartment: IUnsecuredApartment = unsafe {
                CoCreateInstance(&UnsecuredApartment, None, CLSCTX_LOCAL_SERVER)
                    .context("Failed to CoCreateInstance for IUnsecuredApartment")?
            };

            let (event_sink, called) = EventSink::new(&processes, Box::new(cb.clone()));
            let event_sink: IWbemObjectSink = event_sink.into();

            let stub_sink: IWbemObjectSink = unsafe {
                unsecured_apartment
                    .CreateObjectStub(&event_sink)
                    .context("Failed to create unsecured apartment CreateObjectStub")?
                    .cast()
                    .context("Failed to cast unsecured apartment for CreateObjectStub")?
            };

            let query = format!("SELECT TargetInstance.ExecutablePath FROM __InstanceCreationEvent WITHIN 1 WHERE TargetInstance ISA 'Win32_Process' AND ({})", {
                let mut instances = Vec::new();
                for process in processes {
                    instances.push(format!("TargetInstance.ExecutablePath = '{}'", process.display().to_string().replace('\\', r"\\")));
                }

                instances.join(" OR ")
            });

            unsafe {
                services
                    .ExecNotificationQueryAsync(
                        &BSTR::from("WQL"),
                        &BSTR::from(query),
                        WBEM_FLAG_SEND_STATUS,
                        None,
                        &stub_sink,
                    )
                    .context("Failed to ExecNotificationQueryAsync")?;
            }

            let res = unsafe { WaitForSingleObject(event_handle, timeout_ms) };

            // These are the two valid events, so anything other than these is invalid
            if res != WAIT_OBJECT_0 && res != WAIT_TIMEOUT {
                panic!("WaitForSingleObject failed: {res:?}");
            }

            if res == WAIT_TIMEOUT && !called.load(Ordering::Relaxed) {
                cb(CallType::Timeout);
            }

            anyhow::Result::Ok(())
        });

        Ok(Self {
            event,
            thread: Mutex::new(Some(thread)),
        })
    }
}

impl Drop for ProcessWatcher {
    fn drop(&mut self) {
        self.stop();

        if let Ok(mut guard) = self.thread.lock() {
            if let Some(thread) = guard.take() {
                _ = thread.join();
            }
        }
    }
}
