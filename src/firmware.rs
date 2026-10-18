//! Read-only firmware research boundary. No device handles, claims, raw CDBs,
//! update requests, file copies to a player, or updater subprocesses exist here.
use anyhow::{Result, ensure};
use des::{
    Des,
    cipher::{BlockDecrypt, KeyInit, generic_array::GenericArray},
};
use nusb::MaybeFuture;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

const MAX_PACKAGE: u64 = 64 * 1024 * 1024;
const STOCK_SHA256: &str = "cfbcc372e2a18f11c12a080c9e5f039333e09175f5db03ac841a4227d3896b32";

/// An offline specification, NOT an executable transport request. Corroborated
/// by FWUpdaterCom 0x1000a180 and mgscsi vscProcessCmd 0x11b8c / DEVINFO.
#[derive(Debug, Serialize)]
pub struct DeviceInfoQuery {
    pub cdb: [u8; 12],
    pub response_bytes: usize,
    pub live_enabled: bool,
}

impl DeviceInfoQuery {
    fn researched() -> Self {
        Self {
            cdb: [0xfc, 0, 0x20, b'd', b'b', b'm', b'n', 0, 0x80, 0, 0, 0],
            response_bytes: 128,
            live_enabled: false,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UsbCandidate {
    pub vendor_id: String,
    pub product_id: String,
    pub product: Option<String>,
    pub serial: Option<String>,
    pub bus: String,
    pub ports: Vec<u8>,
    pub speed: Option<String>,
    /// bcdDevice is NOT the installed firmware version.
    pub usb_device_revision: String,
    pub model_verified: bool,
}

#[derive(Debug, Serialize)]
pub struct ProbeReport {
    pub platform: &'static str,
    pub mode: &'static str,
    pub firmware_write_enabled: bool,
    pub vendor_queries_enabled: bool,
    pub researched_device_info_query: DeviceInfoQuery,
    pub installed_firmware: Option<String>,
    pub battery_percent: Option<u8>,
    pub candidates: Vec<UsbCandidate>,
}

pub fn probe() -> Result<ProbeReport> {
    let candidates = nusb::list_devices()
        .wait()?
        .filter(|d| d.vendor_id() == 0x054c && d.product_id() == 0x032a)
        .map(|d| UsbCandidate {
            vendor_id: format!("{:04X}", d.vendor_id()),
            product_id: format!("{:04X}", d.product_id()),
            product: d.product_string().map(str::to_owned),
            serial: d.serial_number().map(str::to_owned),
            bus: d.bus_id().to_string(),
            ports: d.port_chain().to_vec(),
            speed: d.speed().map(|s| format!("{s:?}")),
            usb_device_revision: format!("{:04X}", d.device_version()),
            // The stock INI shares this VID/PID across A916/A918/A919.
            model_verified: false,
        })
        .collect();
    Ok(ProbeReport {
        platform: std::env::consts::OS,
        mode: "usb_inventory_only",
        firmware_write_enabled: false,
        vendor_queries_enabled: false,
        researched_device_info_query: DeviceInfoQuery::researched(),
        installed_firmware: None,
        battery_percent: None,
        candidates,
    })
}

#[derive(Debug, Serialize)]
pub struct Component {
    pub index: usize,
    pub offset: usize,
    pub bytes: usize,
    pub sha256: String,
}

#[derive(Debug, Serialize)]
pub struct PackageReport {
    pub bytes: usize,
    pub sha256: String,
    pub outer_md5_valid: bool,
    pub family_signature: String,
    pub stock_103_exact_match: bool,
    pub components: Vec<Component>,
    pub authenticity_verified: bool,
    pub device_compatibility_verified: bool,
    pub firmware_write_enabled: bool,
}

/// Only open an existing regular file for reading; cap allocation even if it grows.
pub fn inspect(path: &Path) -> Result<PackageReport> {
    let file = File::open(path)?;
    let meta = file.metadata()?;
    ensure!(meta.is_file(), "Firmware input must be a regular file");
    ensure!(meta.len() <= MAX_PACKAGE, "Firmware package exceeds 64 MiB");
    let mut bytes = Vec::new();
    file.take(MAX_PACKAGE + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_PACKAGE,
        "Firmware package exceeds 64 MiB"
    );
    inspect_bytes(&bytes)
}

fn decrypt(bytes: &[u8]) -> Vec<u8> {
    let cipher = Des::new(GenericArray::from_slice(b"b8442c44"));
    let mut plain = bytes.to_vec();
    for block in plain.chunks_exact_mut(8) {
        cipher.decrypt_block(GenericArray::from_mut_slice(block));
    }
    plain
}

fn le32(bytes: &[u8]) -> usize {
    u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize
}

fn inspect_bytes(bytes: &[u8]) -> Result<PackageReport> {
    ensure!(
        bytes.len() >= 64 && bytes.len() as u64 <= MAX_PACKAGE && bytes.len().is_multiple_of(8),
        "Invalid UPG length"
    );
    ensure!(
        bytes[..16] == md5::compute(&bytes[16..]).0,
        "UPG outer MD5 mismatch"
    );
    let header = decrypt(&bytes[16..64]);
    ensure!(
        &header[..8] == b"5dfeff3c",
        "Not the researched A91x UPG family"
    );
    ensure!(
        le32(&header[8..]) == 4 && le32(&header[12..]) == 0,
        "Unsupported UPG component table"
    );
    let mut ranges = Vec::new();
    let mut components = Vec::new();
    for index in 0..4 {
        let offset = le32(&header[16 + index * 8..]);
        let size = le32(&header[20 + index * 8..]);
        let padded = size
            .checked_add(7)
            .map(|n| n & !7)
            .ok_or_else(|| anyhow::anyhow!("UPG size overflow"))?;
        let end = offset
            .checked_add(padded)
            .ok_or_else(|| anyhow::anyhow!("UPG offset overflow"))?;
        ensure!(
            size > 0 && offset >= 64 && offset.is_multiple_of(8) && end <= bytes.len(),
            "UPG component outside package"
        );
        ensure!(
            ranges
                .iter()
                .all(|&(start, stop)| end <= start || offset >= stop),
            "Overlapping UPG components"
        );
        ranges.push((offset, end));
        let plain = decrypt(&bytes[offset..end]);
        components.push(Component {
            index,
            offset,
            bytes: size,
            sha256: format!("{:x}", Sha256::digest(&plain[..size])),
        });
    }
    let sha256 = format!("{:x}", Sha256::digest(bytes));
    Ok(PackageReport {
        bytes: bytes.len(),
        stock_103_exact_match: sha256 == STOCK_SHA256,
        sha256,
        outer_md5_valid: true,
        family_signature: "5dfeff3c".into(),
        components,
        // MD5 and DES are format checks, not publisher authentication or model checks.
        authenticity_verified: false,
        device_compatibility_verified: false,
        firmware_write_enabled: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use des::cipher::BlockEncrypt;

    #[test]
    fn device_info_spec_matches_both_host_and_device_checks_and_stays_offline() {
        let query = DeviceInfoQuery::researched();
        assert_eq!(
            query.cdb,
            [0xfc, 0, 0x20, 0x64, 0x62, 0x6d, 0x6e, 0, 0x80, 0, 0, 0]
        );
        assert_eq!(query.response_bytes, 128);
        assert!(!query.live_enabled);
        // The legacy updater's subcommand 0x04 is intentionally not represented.
        assert_ne!(query.cdb[2], 4);
    }

    fn fixture() -> Vec<u8> {
        let mut bytes = vec![0; 96];
        bytes[16..24].copy_from_slice(b"5dfeff3c");
        bytes[24..28].copy_from_slice(&4u32.to_le_bytes());
        for i in 0..4 {
            bytes[32 + i * 8..36 + i * 8].copy_from_slice(&(64u32 + i as u32 * 8).to_le_bytes());
            bytes[36 + i * 8..40 + i * 8].copy_from_slice(&8u32.to_le_bytes());
        }
        seal(&mut bytes);
        bytes
    }
    fn seal(bytes: &mut [u8]) {
        let cipher = Des::new(GenericArray::from_slice(b"b8442c44"));
        for block in bytes[16..].chunks_exact_mut(8) {
            cipher.encrypt_block(GenericArray::from_mut_slice(block));
        }
        let digest = md5::compute(&bytes[16..]);
        bytes[..16].copy_from_slice(&digest.0);
    }
    #[test]
    fn valid_container_does_not_authorize_writing_or_claim_authenticity() {
        let r = inspect_bytes(&fixture()).unwrap();
        assert_eq!(r.components.len(), 4);
        assert!(
            !r.firmware_write_enabled
                && !r.authenticity_verified
                && !r.device_compatibility_verified
                && !r.stock_103_exact_match
        );
    }
    #[test]
    fn reject_corruption_truncation_and_unsupported_headers() {
        let good = fixture();
        for n in 0..good.len() {
            assert!(inspect_bytes(&good[..n]).is_err());
        }
        let mut bad = good;
        bad[90] ^= 1;
        assert!(inspect_bytes(&bad).is_err());
        let mut bad = decrypt(&bad[16..]);
        bad[..8].fill(0);
        let mut whole = vec![0; 16];
        whole.extend(bad);
        seal(&mut whole);
        assert!(inspect_bytes(&whole).is_err());
    }
    #[test]
    fn reject_overlap_and_out_of_bounds_even_with_valid_checksum() {
        for offset in [0u32, 63, 64, u32::MAX] {
            let original = fixture();
            let mut bytes = vec![0; 16];
            bytes.extend(decrypt(&original[16..]));
            bytes[40..44].copy_from_slice(&offset.to_le_bytes());
            seal(&mut bytes);
            assert!(inspect_bytes(&bytes).is_err());
        }
    }
    #[test]
    fn inspecting_file_preserves_source_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("firmware.upg");
        let bytes = fixture();
        std::fs::write(&path, &bytes).unwrap();
        assert!(inspect(&path).is_ok());
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        assert!(inspect(dir.path()).is_err());
    }
}
