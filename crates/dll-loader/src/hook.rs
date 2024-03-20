use std::{ffi::c_int, sync::Arc};

use crossbeam::atomic::AtomicCell;
use eyre::Result;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

use crate::{iat::IATHook, paths::get_dll_dir_filepath, plugin_loader, popup::fatal_popup};

type HookSig = extern "C" fn(
    *const Option<unsafe extern "C" fn() -> c_int>,
    *const unsafe extern "C" fn() -> c_int,
) -> c_int;

static HOOK: AtomicCell<Option<Arc<IATHook<HookSig>>>> = AtomicCell::new(None);

pub fn hook() -> Result<()> {
    if let Some(hook) = HOOK.take() {
        hook.install()?;
        return Ok(());
    };

    let hook = unsafe {
        IATHook::new(
            "api-ms-win-crt-runtime-l1-1-0.dll",
            "_initterm_e",
            _initterm_e as HookSig,
        )
    };

    hook.install()?;

    HOOK.store(Some(Arc::new(hook)));

    Ok(())
}

extern "C" fn _initterm_e(
    ppfn: *const Option<unsafe extern "C" fn() -> c_int>,
    end: *const unsafe extern "C" fn() -> c_int,
) -> c_int {
    let module = unsafe { GetModuleHandleW(None) };
    let Ok(module) = module else {
        fatal_popup(
            "Yet Another BG3 Mod Loader Error",
            "_initterm_e: Failed to get module handle",
        );
    };

    let Ok(plugins_dir) = get_dll_dir_filepath(module.into(), "NativeMods") else {
        fatal_popup(
            "Yet Another BG3 Mod Loader Error",
            "_initterm_e: Failed to get dll directory path",
        );
    };

    if let Err(e) = plugin_loader::load(&plugins_dir) {
        fatal_popup(
            "Yet Another BG3 Mod Loader Error",
            format!("_initterm_e: Failed to load plugins\n\n{e}"),
        );
    }

    let hook = HOOK.take().unwrap();

    log::info!("Running hook fn");

    (hook.call())(ppfn, end)
}
