fn main() {
    if !cfg!(target_os = "windows") {
        panic!("Only windows OS is supported");
    }

    winres::WindowsResource::new().compile().unwrap();
}
