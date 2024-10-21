use std::error::Error;
use std::{env, fs, time::UNIX_EPOCH};
use std::{path::Path, time::SystemTime};

fn main() -> Result<(), Box<dyn Error>> {
    if !cfg!(target_os = "windows") {
        panic!("Only windows OS is supported");
    }

    let mut res = winres::WindowsResource::new();
    // ordinal 1
    res.set_icon("icon.ico");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = Path::new(&manifest_dir);
    let manifest = fs::read_to_string(dir.join("manifest.xml"))?;
    res.set_manifest(&manifest);

    res.compile()?;

    calc_hash()?;

    env::set_var(
        "REBUILD",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string(),
    );

    println!("cargo:rerun-if-env-changed=REBUILD");

    Ok(())
}

fn calc_hash() -> Result<(), Box<dyn Error>> {
    // unfortunately RA needs this workaround to build locally

    let env = env::var_os("OUT_DIR").unwrap();

    let root = Path::new(&env)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let loader_dll = root.join("deps").join("loader.dll");

    let data = fs::read(&loader_dll)?;

    let hash = sha256::digest(&data);

    println!("cargo::rustc-env=LOADER_HASH={hash}");

    // we want this in the main folder tho
    fs::copy(&loader_dll, root.join("loader.dll")).unwrap();

    Ok(())
}
