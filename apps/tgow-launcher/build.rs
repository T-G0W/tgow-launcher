fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("../../assets/brand/tgow-launcher.ico")
        .set("ProductName", "T-GOW Launcher")
        .set("FileDescription", "T-GOW Launcher")
        .set("CompanyName", "T-GOW")
        .set(
            "LegalCopyright",
            "Copyright (c) 2026 T-GOW. All rights reserved.",
        )
        .set("OriginalFilename", "tgow-launcher.exe")
        .set("InternalName", "tgow-launcher");
    resource
        .compile()
        .expect("windows resources could not be compiled");
}
