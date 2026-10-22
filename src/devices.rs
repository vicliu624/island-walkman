//! Device support is explicit data, independent of discovery and UI branding.
//! USB matches identify a family candidate, never an exact model or flash permission.
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DeviceProfile {
    pub id: &'static str,
    pub label: &'static str,
    pub vendor_id: u16,
    pub product_id: u16,
    pub vendor: &'static str,
    pub models: &'static [&'static str],
    /// Offline transaction evidence only; never enables a live firmware transport.
    pub offline_firmware_models: &'static [&'static str],
}

pub const PROFILES: &[DeviceProfile] = &[DeviceProfile {
    id: "sony-a910",
    label: "Sony NW-A910",
    vendor_id: 0x054c,
    product_id: 0x032a,
    vendor: "SONY",
    models: &["NW-A916", "NW-A918", "NW-A919"],
    offline_firmware_models: &["NW-A918"],
}];

pub fn usb_profile(vendor: u16, product: u16) -> Option<&'static DeviceProfile> {
    PROFILES
        .iter()
        .find(|p| p.vendor_id == vendor && p.product_id == product)
}

/// Package-family metadata for offline parsing and read-only identity checks.
/// This does not grant firmware-write permission or hardware qualification.
pub struct FirmwareProfile {
    pub package_signature: &'static str,
}

pub const A910_FIRMWARE: FirmwareProfile = FirmwareProfile {
    package_signature: "5dfeff3c",
};

pub fn firmware_profile(vendor: &str, model: &str) -> Option<&'static FirmwareProfile> {
    PROFILES
        .iter()
        .any(|profile| profile.vendor == vendor && profile.models.contains(&model))
        .then_some(&A910_FIRMWARE)
}

pub fn offline_firmware_supported(vendor: &str, model: &str) -> bool {
    PROFILES
        .iter()
        .any(|p| p.vendor == vendor && p.offline_firmware_models.contains(&model))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_does_not_grant_firmware_support() {
        let p = usb_profile(0x054c, 0x032a).unwrap();
        assert_eq!(p.models.len(), 3);
        assert!(!offline_firmware_supported("SONY", "NW-A919"));
        assert!(!offline_firmware_supported("other", "NW-A918"));
        assert!(usb_profile(0x054c, 0xffff).is_none());
    }
}
