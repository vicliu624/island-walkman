//! Decoders for supplied response captures; this module sends no commands.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadQuery {
    DeviceInfo,
    PowerStatus,
}

impl ReadQuery {
    pub fn cdb(self) -> [u8; 12] {
        match self {
            Self::DeviceInfo => [0xfc, 0, 0x20, b'd', b'b', b'm', b'n', 0, 128, 0, 0, 0],
            Self::PowerStatus => [0xfb, 0, 0, b'P', b'W', b'_', b'S', b'T', b'A', b'T', 32, 0],
        }
    }
    pub fn response_len(self) -> usize {
        match self {
            Self::DeviceInfo => 128,
            Self::PowerStatus => 32,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
}
impl Version {
    pub fn parse(text: &str) -> Result<Self> {
        let b = text.as_bytes();
        ensure!(
            b.len() == 4
                && b[0].is_ascii_digit()
                && b[1] == b'.'
                && b[2].is_ascii_digit()
                && b[3].is_ascii_digit(),
            "Unsupported A91x firmware revision"
        );
        Ok(Self {
            major: b[0] - b'0',
            minor: (b[2] - b'0') * 10 + b[3] - b'0',
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceInfo {
    pub vendor: String,
    pub model: String,
    pub version: Version,
    pub revision: String,
    pub subrevision: String,
    /// Preserve uninterpreted serial bytes; do not equate them with USB serial.
    pub device_serial_hex: String,
    pub storage_size_hex: String,
}

impl DeviceInfo {
    /// Diagnostics include derived branding without changing persisted checkpoint data.
    pub fn report(&self) -> Result<serde_json::Value> {
        let mut report = serde_json::to_value(self)?;
        report["firmware_identity"] = serde_json::to_value(super::identity::identify(self))?;
        Ok(report)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PowerStatus {
    pub supply_status_raw: u8,
    pub battery_status_raw: u8,
    pub percent: Option<u8>,
    pub life_seconds: Option<u32>,
    pub full_life_seconds: Option<u32>,
}

fn text(bytes: &[u8]) -> Result<String> {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    ensure!(
        bytes[end..].iter().all(|b| *b == 0 || *b == b' '),
        "Invalid response string padding"
    );
    ensure!(
        bytes[..end].iter().all(|b| (0x20..=0x7e).contains(b)),
        "Non-ASCII response identity"
    );
    Ok(std::str::from_utf8(&bytes[..end])?.trim_end().to_owned())
}
fn header(bytes: &[u8], flag: &[u8], len: usize) -> Result<()> {
    ensure!(bytes.len() == len, "Unexpected firmware response length");
    ensure!(
        &bytes[..7] == flag && bytes[7] as usize == len,
        "Unexpected firmware response header"
    );
    Ok(())
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|v| format!("{v:02x}")).collect()
}
pub fn device_info(bytes: &[u8]) -> Result<DeviceInfo> {
    header(bytes, b"DEVINFO", 128)?;
    let revision = text(&bytes[40..44])?;
    Ok(DeviceInfo {
        vendor: text(&bytes[16..24])?,
        model: text(&bytes[24..40])?,
        version: Version::parse(&revision)?,
        revision,
        subrevision: text(&bytes[44..48])?,
        device_serial_hex: hex(&bytes[52..68]),
        storage_size_hex: hex(&bytes[48..52]),
    })
}
pub fn power_status(bytes: &[u8]) -> Result<PowerStatus> {
    header(bytes, b"PW_STAT", 32)?;
    let percent = match bytes[11] {
        0..=100 => Some(bytes[11]),
        255 => None,
        _ => anyhow::bail!("Invalid battery percentage"),
    };
    let seconds = |start| {
        let n = u32::from_be_bytes(bytes[start..start + 4].try_into().unwrap());
        if n == u32::MAX { None } else { Some(n) }
    };
    Ok(PowerStatus {
        supply_status_raw: bytes[9],
        battery_status_raw: bytes[10],
        percent,
        life_seconds: seconds(12),
        full_life_seconds: seconds(16),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn info_fixture() -> Vec<u8> {
        let mut b = vec![0; 128];
        b[..7].copy_from_slice(b"DEVINFO");
        b[7] = 128;
        b[16..24].copy_from_slice(b"SONY    ");
        b[24..40].copy_from_slice(b"NW-A918         ");
        b[40..44].copy_from_slice(b"1.03");
        b[44..48].copy_from_slice(b"00  ");
        b
    }
    pub fn power_fixture() -> Vec<u8> {
        let mut b = vec![0; 32];
        b[..7].copy_from_slice(b"PW_STAT");
        b[7] = 32;
        b[9] = 1;
        b[10] = 8;
        b[11] = 95;
        b[12..16].copy_from_slice(&3600u32.to_be_bytes());
        b[16..20].fill(255);
        b
    }
    #[test]
    fn independently_recorded_typelib_offsets_and_wire_byte_order() {
        let r = device_info(&info_fixture()).unwrap();
        assert_eq!(r.model, "NW-A918");
        assert_eq!(r.vendor, "SONY");
        assert_eq!(r.version, Version { major: 1, minor: 3 });
        assert_eq!(r.subrevision, "00");
        let p = power_status(&power_fixture()).unwrap();
        assert_eq!(p.percent, Some(95));
        assert_eq!(p.life_seconds, Some(3600));
        assert_eq!(p.full_life_seconds, None);
        assert_eq!(p.battery_status_raw, 8);
    }
    #[test]
    fn unknown_and_invalid_battery_values_cannot_become_ready() {
        let mut b = power_fixture();
        b[11] = 255;
        assert_eq!(power_status(&b).unwrap().percent, None);
        for value in 101..255 {
            b[11] = value;
            assert!(power_status(&b).is_err());
        }
    }
    #[test]
    fn malformed_frames_and_identity_strings_are_rejected() {
        let b = info_fixture();
        for len in 0..128 {
            assert!(device_info(&b[..len]).is_err());
        }
        let mut wrong = b.clone();
        wrong.push(0);
        assert!(device_info(&wrong).is_err());
        for offset in [0, 6, 7, 24, 40] {
            let mut wrong = b.clone();
            wrong[offset] = 255;
            assert!(device_info(&wrong).is_err());
        }
        let mut wrong = b;
        wrong[27] = 0;
        assert!(device_info(&wrong).is_err());
        let p = power_fixture();
        for len in 0..32 {
            assert!(power_status(&p[..len]).is_err());
        }
        assert!(Version::parse("1.3").is_err());
        assert!(Version::parse("1.03x").is_err());
        assert!(Version::parse("9.99").is_ok());
    }
    #[test]
    fn read_queries_are_exact_and_have_no_activation_variant() {
        assert_eq!(
            ReadQuery::DeviceInfo.cdb(),
            [252, 0, 32, 100, 98, 109, 110, 0, 128, 0, 0, 0]
        );
        assert_eq!(
            ReadQuery::PowerStatus.cdb(),
            [251, 0, 0, 80, 87, 95, 83, 84, 65, 84, 32, 0]
        );
        for query in [ReadQuery::DeviceInfo, ReadQuery::PowerStatus] {
            assert_ne!(query.cdb()[2], 4);
            assert!(query.response_len() <= 128);
        }
    }
}
