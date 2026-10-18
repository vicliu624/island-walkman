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
    let firmware_operations = [
        "--firmware-readonly-probe",
        "--firmware-safety",
        "--inspect-firmware-response",
        "--firmware-probe",
        "--inspect-firmware",
    ];
    anyhow::ensure!(
        args.iter()
            .filter(|arg| firmware_operations.contains(&arg.as_str()))
            .count()
            <= 1,
        "Select exactly one firmware diagnostic operation"
    );
    if let Some(index) = args
        .iter()
        .position(|arg| arg == "--firmware-readonly-probe")
    {
        let volume = args
            .get(index + 1)
            .ok_or_else(|| anyhow::anyhow!("Missing volume"))?;
        let serial = args
            .iter()
            .position(|arg| arg == "--usb-serial")
            .and_then(|i| args.get(i + 1))
            .ok_or_else(|| anyhow::anyhow!("Missing explicit USB serial"))?;
        #[cfg(windows)]
        println!(
            "{}",
            serde_json::to_string_pretty(&firmware::windows_readonly::probe(volume, serial)?)?
        );
        #[cfg(not(windows))]
        anyhow::bail!(
            "Fixed storage queries are not yet implemented on this platform: {volume}, {serial}"
        );
        #[cfg(windows)]
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--firmware-safety") {
        println!(
            "{}",
            serde_json::to_string_pretty(&firmware::session::live_gate())?
        );
        return Ok(());
    }
    if let Some(index) = args
        .iter()
        .position(|arg| arg == "--inspect-firmware-response")
    {
        let kind = args
            .get(index + 1)
            .ok_or_else(|| anyhow::anyhow!("Missing response type"))?;
        let query = match kind.as_str() {
            "devinfo" => firmware::responses::ReadQuery::DeviceInfo,
            "power" => firmware::responses::ReadQuery::PowerStatus,
            _ => anyhow::bail!("Unsupported response type"),
        };
        let path = args
            .get(index + 2)
            .ok_or_else(|| anyhow::anyhow!("Missing response capture path"))?;
        use std::io::Read;
        let file = std::fs::File::open(path)?;
        anyhow::ensure!(
            file.metadata()?.is_file(),
            "Response capture must be a regular file"
        );
        let mut bytes = Vec::new();
        file.take(query.response_len() as u64 + 1)
            .read_to_end(&mut bytes)?;
        let value = match query {
            firmware::responses::ReadQuery::DeviceInfo => {
                serde_json::to_value(firmware::responses::device_info(&bytes)?)?
            }
            firmware::responses::ReadQuery::PowerStatus => {
                serde_json::to_value(firmware::responses::power_status(&bytes)?)?
            }
        };
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"source":"supplied_capture_not_live_device","decoded":value,"safety":firmware::session::live_gate()})
            )?
        );
        return Ok(());
    }
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
