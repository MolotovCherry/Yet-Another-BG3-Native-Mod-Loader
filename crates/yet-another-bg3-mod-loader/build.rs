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
    // https://github.com/rust-lang/cargo/issues/9096
    // https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#artifact-dependencies-environment-variables
    //
    // note, the dll here is target/*/deps/artifact/loader-*/cdylib/loader.dll, NOT target/loader.dll
    let env = env::var_os("CARGO_CDYLIB_FILE_LOADER").unwrap();
    let data = fs::read(&env)?;

    let hash = sha256::digest(&data);

    println!("cargo::rustc-env=LOADER_HASH={hash}");

    Ok(())
}
