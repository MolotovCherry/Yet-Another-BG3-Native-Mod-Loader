use std::error::Error;

static MANIFEST: &str = include_str!("manifest.xml");

fn main() -> Result<(), Box<dyn Error>> {
    if !cfg!(target_os = "windows") {
        panic!("Only windows OS is supported");
    }

    let mut res = winres::WindowsResource::new();
    // ordinal 1
    res.set_icon("icon.ico");

    res.set_manifest(MANIFEST);

    res.compile()?;

    println!("cargo::rerun-if-env-changed=LOADER_HASH");

    Ok(())
}
