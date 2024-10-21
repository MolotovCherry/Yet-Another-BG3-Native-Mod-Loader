use std::error::Error;
use std::path::Path;
use std::{env, fs};

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

    Ok(())
}

fn calc_hash() -> Result<(), Box<dyn Error>> {
    let env = env::var_os("OUT_DIR").unwrap();
    let loader_dll = Path::new(&env)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("loader.dll");

    let data = fs::read(&loader_dll)?;

    let hash = sha256::digest(&data);

    println!("cargo::rustc-env=LOADER_HASH={hash}");

    Ok(())
}
