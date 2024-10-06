use std::error::Error;
use std::path::{Path, PathBuf};
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
    let data = fs::read(&env)?;

    let hash = sha256::digest(&data);

    let out_dir = {
        let dir = env::var("OUT_DIR")?;
        PathBuf::from(dir)
    };

    let file = out_dir.join("loader.bin");

    let mut out_file = fs::File::create(&file)?;
    zstd::stream::copy_encode(&*data, &mut out_file, 22)?;

    println!("cargo::rustc-env=LOADER_BIN={}", file.display());
    println!("cargo::rustc-env=LOADER_BIN_HASH={}", &hash[..8]);

    Ok(())
}
