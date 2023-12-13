use windows::{
    core::{ComInterface, BSTR},
    Win32::System::{
        Com::{
            CoCreateInstance, CoInitializeSecurity, CoSetProxyBlanket, CLSCTX_INPROC_SERVER,
            CLSCTX_LOCAL_SERVER, COINIT_DISABLE_OLE1DDE, COINIT_MULTITHREADED, EOAC_NONE,
            RPC_C_AUTHN_LEVEL_CALL, RPC_C_AUTHN_LEVEL_DEFAULT, RPC_C_IMP_LEVEL_IMPERSONATE,
        },
        Rpc::{RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE},
        Threading::{CreateEventW, WaitForSingleObject, INFINITE},
        Wmi::{
            IUnsecuredApartment, IWbemLocator, IWbemObjectSink, UnsecuredApartment, WbemLocator,
            WBEM_FLAG_SEND_STATUS,
        },
    },
};

use crate::helpers::OwnedHandle;

use super::{apartment::Apartment, event_sink::EventSink};

pub struct ProcessWatcher;

impl ProcessWatcher {
    pub fn watch_for<F: Fn(u32) + 'static>(processes: &[&str], cb: F) -> windows::core::Result<()> {
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
            )?;
        }

        let locator: IWbemLocator =
            unsafe { CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER)? };
        let services = unsafe {
            locator.ConnectServer(&BSTR::from("ROOT\\CIMV2"), None, None, None, 0, None, None)?
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
            )?;
        }

        let unsecured_apartment: IUnsecuredApartment =
            unsafe { CoCreateInstance(&UnsecuredApartment, None, CLSCTX_LOCAL_SERVER)? };

        let event_sink: IWbemObjectSink = EventSink::new(processes, cb).into();
        let stub_sink: IWbemObjectSink =
            unsafe { unsecured_apartment.CreateObjectStub(&event_sink)?.cast()? };

        unsafe {
            services.ExecNotificationQueryAsync(
                &BSTR::from("WQL"),
                &BSTR::from("SELECT * FROM __InstanceCreationEvent WITHIN 1 WHERE TargetInstance ISA 'Win32_Process'"),
                WBEM_FLAG_SEND_STATUS,
                None,
                &stub_sink,
            )?;
        }

        let event: OwnedHandle = unsafe { CreateEventW(None, false, false, None)?.into() };

        unsafe {
            WaitForSingleObject(event.as_raw_handle(), INFINITE);
        }

        Ok(())
    }
}
