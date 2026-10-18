#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod database;
#[cfg(feature = "desktop")]
mod desktop;
mod device_io;
mod firmware;
mod metadata;
mod protocol;
mod reliable_transfer;
mod service;
mod sony_artwork;
mod space;
fn main() -> anyhow::Result<()> {
    // Research commands exit before desktop startup, catalog loading or recovery.
    let args: Vec<_> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--firmware-probe") {
        println!("{}", serde_json::to_string_pretty(&firmware::probe()?)?);
        return Ok(());
    }
    if let Some(index) = args.iter().position(|arg| arg == "--inspect-firmware") {
        let path = args
            .get(index + 1)
            .ok_or_else(|| anyhow::anyhow!("Missing firmware path"))?;
        println!(
            "{}",
            serde_json::to_string_pretty(&firmware::inspect(std::path::Path::new(path))?)?
        );
        return Ok(());
    }
    #[cfg(feature = "desktop")]
    if !std::env::args().any(|arg| arg == "--headless" || arg == "--prepare") {
        return desktop::run();
    }
    service::run()
}
