use std::error::Error;

static MANIFEST: &str = include_str!("manifest.xml");

fn main() -> Result<(), Box<dyn Error>> {
    if !cfg!(target_os = "windows") {
        panic!("Only windows OS is supported");
    }

    let mut res = winres::WindowsResource::new();

    res.set_manifest(MANIFEST);

    res.compile()?;

    Ok(())
}
