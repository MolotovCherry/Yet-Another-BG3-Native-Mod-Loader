use std::os::windows::{io::IntoRawHandle, prelude::OsStrExt};
use std::path::Path;
use std::{collections::HashMap, fs::OpenOptions, os::windows::fs::OpenOptionsExt as _};

use eyre::{anyhow, Result};
use shared::paths::get_bg3_plugins_dir;
use tracing::{trace, trace_span};
use widestring::U16Str;
use windows::Win32::Storage::FileSystem::{
    FileIdInfo, GetFileInformationByHandleEx, FILE_FLAG_BACKUP_SEMANTICS, FILE_ID_INFO,
    FILE_SHARE_READ,
};

use crate::{
    helpers::OwnedHandle,
    wapi::{enum_modules::enum_modules, get_module_file_name_ex::get_module_file_name_ex_w},
};

#[derive(Debug, Copy, Clone, PartialEq)]
struct Id(u64, u128);

fn dir_id(path: &Path) -> Option<Id> {
    if !path.is_dir() {
        return None;
    }

    // abuse OpenOptions to call CreateFileW with the correct args to get a dir handle
    // this lets us avoid an unsafe call
    let dir = OpenOptions::new()
        .access_mode(0)
        .share_mode(FILE_SHARE_READ.0)
        .attributes(FILE_FLAG_BACKUP_SEMANTICS.0)
        // (self.create, self.truncate, self.create_new) {
        //    (false, false, false) => c::OPEN_EXISTING,
        .create(false)
        .truncate(false)
        .create_new(false)
        .open(path)
        .ok()?;

    let handle: OwnedHandle = dir.into_raw_handle().into();

    let mut info = FILE_ID_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            handle.as_raw_handle(),
            FileIdInfo,
            &mut info as *mut _ as *mut _,
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
        .ok()?;
    }

    let file_id = u128::from_le_bytes(info.FileId.Identifier);

    trace!(path = %path.display(), volume_id = info.VolumeSerialNumber, file_id, "dir id");

    Some(Id(info.VolumeSerialNumber, file_id))
}

// Determine whether the process has been tainted by previous dll injections
pub fn is_dirty(process: &OwnedHandle, loader: &Path) -> Result<bool> {
    let span = trace_span!("is_dirty");
    let _guard = span.enter();

    let loader = loader.as_os_str().encode_wide().collect::<Vec<_>>();
    let loader = U16Str::from_slice(&loader);

    let plugins_dir = get_bg3_plugins_dir()?;

    let mut cache_id_map = HashMap::new();

    trace!(plugins_dir = %plugins_dir.display(), "checking dll path against dirs");

    let plugins_dir_id = dir_id(&plugins_dir).ok_or(anyhow!("failed to get id for plugins_dir"))?;
    cache_id_map.insert(
        plugins_dir.to_string_lossy().to_lowercase().into(),
        plugins_dir_id,
    );

    let mut is_plugin = move |path: &U16Str| -> Result<bool> {
        let path = path.to_string()?;
        let path = Path::new(&path);

        // not a dll file
        if !path.is_file()
            || !path
                .extension()
                .is_some_and(|ext| ext.to_ascii_lowercase() == "dll")
        {
            return Ok(false);
        }

        // get parent dir
        let Some(parent) = path.parent() else {
            return Ok(false);
        };

        let id = if let Some(&id) = cache_id_map.get(parent) {
            id
        } else {
            let Some(path_id) = dir_id(parent) else {
                return Ok(false);
            };

            cache_id_map.insert(parent.to_path_buf(), path_id);

            path_id
        };

        // if plugins dir is the same id as this one, then this is a plugin inside our plugins dir~
        Ok(plugins_dir_id == id)
    };

    let mut detected = false;
    let mut buf = vec![0u16; 1024];
    enum_modules(process, |module| {
        let path = get_module_file_name_ex_w(process, Some(module), &mut buf)?;
        let os_path = path.to_os_string();
        let filename = Path::new(Path::new(&os_path).file_name().unwrap_or_default());

        trace!(module = %filename.display(), "found loaded module");

        if loader == path || is_plugin(path)? {
            detected = true;
            return Ok(false);
        }

        Ok(true)
    })?;

    Ok(detected)
}
