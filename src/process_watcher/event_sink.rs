use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use windows::{
    core::{implement, ComInterface, HRESULT, PCWSTR},
    Win32::{
        Foundation::E_FAIL,
        System::Wmi::{IWbemClassObject, IWbemObjectSink_Impl},
    },
};
use windows::{
    core::{w, BSTR},
    Win32::System::Wmi::IWbemObjectSink,
};

use super::{variant::Variant, watcher::CallType};

pub type SinkCallback = Box<dyn Fn(CallType) + 'static>;

#[implement(IWbemObjectSink)]
pub struct EventSink {
    called: Arc<AtomicBool>,
    processes: Vec<Vec<u16>>,
    cb: SinkCallback,
}

impl EventSink {
    pub fn new(processes: &[&str], cb: SinkCallback) -> (Self, Arc<AtomicBool>) {
        let called = Arc::new(AtomicBool::new(false));

        let sink = Self {
            called: called.clone(),
            processes: processes
                .iter()
                .map(|&s| {
                    let mut data = s.encode_utf16().collect::<Vec<_>>();
                    data.push(0);
                    data
                })
                .collect(),

            cb,
        };

        (sink, called)
    }

    fn get(object: &IWbemClassObject, name: PCWSTR) -> windows::core::Result<Variant> {
        let mut variant = Variant::new();

        unsafe { object.Get(name, 0, variant.as_mut_ptr(), None, None)? };

        Ok(variant)
    }

    fn bstr_equal(object: &IWbemClassObject, name: PCWSTR, string: PCWSTR) -> bool {
        Self::get(object, name).map_or(false, |variant| unsafe {
            variant.Anonymous.Anonymous.Anonymous.bstrVal.as_wide() == string.as_wide()
        })
    }

    fn handle_event(&self, object: &IWbemClassObject) -> windows::core::Result<()> {
        if Self::bstr_equal(object, w!("__Class"), w!("__InstanceCreationEvent")) {
            let target_instance: IWbemClassObject = unsafe {
                Self::get(object, w!("TargetInstance"))?
                    .Anonymous
                    .Anonymous
                    .Anonymous
                    .punkVal
                    .as_ref()
                    .ok_or(E_FAIL)?
                    .cast()?
            };

            for process in &self.processes {
                if Self::bstr_equal(
                    &target_instance,
                    w!("ExecutablePath"),
                    PCWSTR(process.as_ptr()),
                ) {
                    self.called.store(true, Ordering::Relaxed);
                    let pid = self.get_pid(&target_instance)?;
                    (self.cb)(CallType::Pid(pid));
                }
            }

            Ok(())
        } else {
            Ok(())
        }
    }

    fn get_pid(&self, process: &IWbemClassObject) -> windows::core::Result<u32> {
        let pid = unsafe {
            Self::get(process, w!("ProcessId"))?
                .Anonymous
                .Anonymous
                .Anonymous
                .uintVal
        };

        Ok(pid)
    }
}

impl IWbemObjectSink_Impl for EventSink {
    #[allow(non_snake_case)]
    fn Indicate(
        &self,
        object_count: i32,
        object_array: *const Option<IWbemClassObject>,
    ) -> windows::core::Result<()> {
        let objects = unsafe { std::slice::from_raw_parts(object_array, object_count as usize) };

        for object in objects {
            match object {
                Some(object) => self.handle_event(object)?,
                None => continue,
            };
        }

        Ok(())
    }

    #[allow(non_snake_case)]
    fn SetStatus(
        &self,
        _flags: i32,
        _hresult: HRESULT,
        _strparam: &BSTR,
        _pobjparam: Option<&IWbemClassObject>,
    ) -> ::windows::core::Result<()> {
        Ok(())
    }
}
