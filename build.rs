// Embeds the application icon into the Windows executable so the
// .exe file itself carries the RoomForge mark in Explorer, the
// taskbar and shortcuts. Only runs when the build host is Windows
// (CI); on other hosts the runtime window icon in main.rs still
// applies wherever the platform supports it.
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
    if cfg!(windows) {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=icon resource not embedded: {e}");
        }
    }
}
