use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use log::debug;
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

        debug!("Looking for processes:\n{processes:?}");

        let processes = processes
            .iter()
            .map(|&s| {
                let mut data = s.encode_utf16().collect::<Vec<_>>();
                data.push(0);
                data
            })
            .collect();

        debug!("Processes will be looking specifically for bytes:\n{processes:?}");

        let sink = Self {
            called: called.clone(),
            processes,
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
        Self::get(object, name).map_or(false, |variant| {
            let target = unsafe { variant.Anonymous.Anonymous.Anonymous.bstrVal.as_wide() };
            let source = unsafe { string.as_wide() };

            debug!("bstr_equal: comparing src \"{source:?}\" and target \"{target:?}\"");
            debug!("bstr_equal: these are bytes:\nsrc: {source:?}, target: {target:?}");

            target == source
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
                    debug!(
                        "Found newly created executable: {:?}",
                        String::from_utf16_lossy(process)
                    );

                    self.called.store(true, Ordering::Relaxed);
                    let pid = self.get_pid(&target_instance)?;

                    debug!("Event sink got pid {pid}, now calling callback");

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
