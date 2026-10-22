//! Interpretation of firmware-reported branding, independent of Sony's numeric
//! updater version. IWnn is a reserved contract, not yet emitted by a validated
//! firmware build. A declaration is not proof of authenticity or launcher ABI.
use super::responses::DeviceInfo;
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    IslandDeclared,
    Unidentified,
}

#[derive(Debug, Serialize)]
pub struct Identity {
    pub origin: Origin,
    pub base_version: String,
    pub raw_subrevision: String,
    pub custom_revision: Option<u8>,
    pub display_version: String,
    pub exact_build: Option<String>,
    pub launcher_abi: Option<u32>,
    pub authenticity_verified: bool,
}

pub fn identify(device: &DeviceInfo) -> Identity {
    let sub = device.subrevision.as_bytes();
    let known_family = crate::devices::firmware_profile(&device.vendor, &device.model).is_some();
    let custom_revision = if known_family
        && sub.len() == 4
        && &sub[..2] == b"IW"
        && sub[2].is_ascii_digit()
        && sub[3].is_ascii_digit()
    {
        Some((sub[2] - b'0') * 10 + sub[3] - b'0')
    } else {
        None
    };
    Identity {
        origin: if custom_revision.is_some() {
            Origin::IslandDeclared
        } else {
            Origin::Unidentified
        },
        base_version: device.revision.clone(),
        raw_subrevision: device.subrevision.clone(),
        display_version: match custom_revision {
            Some(revision) => format!("Island R{revision} (Sony base {})", device.revision),
            None => format!("{} (distribution unidentified)", device.revision),
        },
        custom_revision,
        // Four subrevision bytes cannot communicate the exact local build or ABI.
        // Never infer these from the R number or a package filename.
        exact_build: None,
        launcher_abi: None,
        authenticity_verified: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::firmware::responses::{device_info, tests::info_fixture};

    #[test]
    fn unchanged_sony_version_cannot_identify_an_old_custom_build_as_stock() {
        let device = device_info(&info_fixture()).unwrap();
        let identity = identify(&device);
        assert_eq!(identity.origin, Origin::Unidentified);
        assert_eq!(identity.custom_revision, None);
        assert_eq!(identity.launcher_abi, None);
    }

    #[test]
    fn declared_custom_revision_is_distinct_from_base_version_and_capabilities() {
        let mut bytes = info_fixture();
        bytes[44..48].copy_from_slice(b"IW14");
        let mut device = device_info(&bytes).unwrap();
        for model in ["NW-A916", "NW-A918", "NW-A919"] {
            device.model = model.into();
            let identity = identify(&device);
            assert_eq!(identity.origin, Origin::IslandDeclared);
            assert_eq!(identity.custom_revision, Some(14));
            assert_eq!(identity.base_version, "1.03");
            assert!(identity.exact_build.is_none());
            assert!(identity.launcher_abi.is_none());
            assert!(!identity.authenticity_verified);
        }
    }

    #[test]
    fn branding_requires_exact_namespace_and_known_family() {
        let mut device = device_info(&info_fixture()).unwrap();
        for subrevision in ["00", "R14", "iw14", "IW1", "IWXX", "IW140"] {
            device.subrevision = subrevision.into();
            assert_eq!(identify(&device).origin, Origin::Unidentified);
        }
        device.subrevision = "IW14".into();
        device.vendor = "OTHER".into();
        assert_eq!(identify(&device).origin, Origin::Unidentified);
    }
}
