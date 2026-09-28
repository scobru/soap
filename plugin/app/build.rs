fn main() {
    // The icon of Soap.exe in Explorer and the taskbar. Only a Windows host has
    // the resource compiler; cross-checks from elsewhere skip it.
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/soap.ico");
        resource.set("ProductName", "Soap");
        resource.set("FileDescription", "Soap voice cleaner");
        resource.compile().expect("could not embed the Windows icon");
    }
    println!("cargo:rerun-if-changed=assets/soap.ico");
}
