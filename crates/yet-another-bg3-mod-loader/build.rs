use std::error::Error;
use std::hash::{DefaultHasher, Hasher};
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

    build_compressed()?;

    Ok(())
}

fn build_compressed() -> Result<(), Box<dyn Error>> {
    // https://github.com/rust-lang/cargo/issues/9096
    // https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#artifact-dependencies-environment-variables
    let env = env::var_os("CARGO_CDYLIB_FILE_LOADER").unwrap();
    let path = Path::new(&env);
    let data = fs::read(path)?;

    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    let data = lz4_flex::compress_prepend_size(&data);

    let file = out_dir.join("loader.bin");
    fs::write(&file, &data)?;

    println!("cargo::rustc-env=LOADER_BIN={}", file.display());

    let hash = sha256::digest(&data);

    println!("cargo::rustc-env=LOADER_BIN_HASH={}", &hash[..8]);

    Ok(())
}
