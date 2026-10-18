#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod database;
#[cfg(feature = "desktop")]
mod desktop;
mod device_io;
mod metadata;
mod protocol;
mod service;
mod sony_artwork;
mod space;
fn main() -> anyhow::Result<()> {
    #[cfg(feature = "desktop")]
    if !std::env::args().any(|arg| arg == "--headless" || arg == "--prepare") {
        return desktop::run();
    }
    service::run()
}
