fn main() {
    if !cfg!(target_os = "windows") {
        panic!("Only windows OS is supported");
    }
    let mut res = winres::WindowsResource::new();
    // ordinal 1
    res.set_icon("icon.ico");

    // allow high dpi scaling
    res.set_manifest(r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0" xmlns:asmv3="urn:schemas-microsoft-com:asm.v3">
    <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
        <application>
            <!-- Windows 10 and Windows 11 -->
            <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
        </application>
    </compatibility>
    <asmv3:application>
        <asmv3:windowsSettings>
        <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true</dpiAware>
        <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">system</dpiAwareness>
        </asmv3:windowsSettings>
    </asmv3:application>
</assembly>
"#);

    res.compile().unwrap();
}
