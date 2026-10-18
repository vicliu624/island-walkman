//! Pure, offline upgrade transaction model. There is deliberately no transport
//! trait implementation, device filesystem access or activation executor here.
#![allow(dead_code)] // Remains disconnected from live devices until safety evidence exists.
use super::responses::{DeviceInfo, PowerStatus, Version};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub fn live_gate() -> serde_json::Value {
    preflight(None, None)
}

pub fn preflight(device: Option<&DeviceInfo>, power: Option<&PowerStatus>) -> serde_json::Value {
    let mut blockers = vec![
        "firmware upload and activation transport not validated",
        "live response semantics not validated",
        "resident installer and recovery path not verified",
        "hardware flashing not authorized",
        "A918 battery threshold and status policy not established",
        "candidate model and version metadata not authenticated",
    ];
    if device.is_none_or(|d| {
        d.vendor != "SONY" || d.model != "NW-A918" || d.device_serial_hex.bytes().all(|b| b == b'0')
    }) {
        blockers.push("exact A918 identity and serial binding not verified");
    }
    if power.is_none_or(|p| p.percent.is_none()) {
        blockers.push("battery percentage unavailable");
    }
    if power.is_some_and(|p| p.percent == Some(0)) {
        blockers.push("battery is empty");
    }
    if power.is_some_and(|p| p.supply_status_raw > 1 || p.battery_status_raw == 255) {
        blockers.push("power status is unknown");
    }
    serde_json::json!({"firmware_write_enabled":false,"raw_vendor_queries_enabled":false,"fixed_read_only_queries_available_on_windows":cfg!(windows),"blockers":blockers})
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Phase {
    Prepared,
    PayloadVerified,
    ActivationIntent,
    AwaitingReconnect,
    AwaitingReadback,
    Completed,
    Review,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Session {
    schema: u8,
    phase: Phase,
    generation: u64,
    identity: DeviceInfo,
    usb_serial: String,
    image_sha256: String,
    target_version: Version,
    activation_intent_recorded: bool,
    review_reason: Option<String>,
    observed_readback: Option<DeviceInfo>,
}

fn digest_valid(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn same_device(a: &DeviceInfo, b: &DeviceInfo) -> bool {
    a.vendor == b.vendor
        && a.model == b.model
        && a.device_serial_hex == b.device_serial_hex
        && a.storage_size_hex == b.storage_size_hex
}

impl Session {
    pub fn new(
        identity: DeviceInfo,
        usb_serial: String,
        image_sha256: String,
        target_version: Version,
    ) -> Result<Self> {
        ensure!(
            identity.vendor == "SONY" && identity.model == "NW-A918",
            "Exact A918 identity required"
        );
        ensure!(
            Version::parse(&identity.revision)? == identity.version,
            "Inconsistent firmware revision"
        );
        ensure!(
            target_version.major <= 9 && target_version.minor <= 99,
            "Unsupported target firmware revision"
        );
        let hex_field = |s: &str, len: usize| {
            s.len() == len
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        ensure!(
            !usb_serial.is_empty()
                && usb_serial.len() <= 128
                && usb_serial.bytes().all(|b| b.is_ascii_graphic())
                && hex_field(&identity.device_serial_hex, 32)
                && hex_field(&identity.storage_size_hex, 8)
                && identity.device_serial_hex.bytes().any(|b| b != b'0'),
            "Device serial binding is unavailable"
        );
        ensure!(
            digest_valid(&image_sha256),
            "Invalid immutable firmware digest"
        );
        ensure!(
            (target_version.major, target_version.minor)
                > (identity.version.major, identity.version.minor),
            "Same-version and downgrade installation remain unverified"
        );
        Ok(Self {
            schema: 1,
            phase: Phase::Prepared,
            generation: 0,
            identity,
            usb_serial,
            image_sha256,
            target_version,
            activation_intent_recorded: false,
            review_reason: None,
            observed_readback: None,
        })
    }
    fn review(&mut self, reason: &str) {
        self.phase = Phase::Review;
        self.review_reason = Some(reason.into());
        self.generation += 1;
    }
    pub fn payload_verified(
        &mut self,
        digest: &str,
        current: &DeviceInfo,
        usb_serial: &str,
    ) -> Result<()> {
        ensure!(
            self.phase == Phase::Prepared,
            "Invalid payload verification transition"
        );
        if digest != self.image_sha256
            || !same_device(&self.identity, current)
            || usb_serial != self.usb_serial
            || current.version != self.identity.version
        {
            self.review("Payload or target identity changed before activation");
            return Ok(());
        }
        self.phase = Phase::PayloadVerified;
        self.generation += 1;
        Ok(())
    }
    /// The caller must durably commit this intent BEFORE any future executor
    /// could send activation. Restoring this phase never grants a resend.
    pub fn record_activation_intent(&mut self) -> Result<()> {
        ensure!(
            self.phase == Phase::PayloadVerified && !self.activation_intent_recorded,
            "Activation cannot be repeated"
        );
        self.activation_intent_recorded = true;
        self.phase = Phase::ActivationIntent;
        self.generation += 1;
        Ok(())
    }
    pub fn activation_acknowledged(&mut self) -> Result<()> {
        ensure!(
            self.phase == Phase::ActivationIntent,
            "Unexpected activation acknowledgement"
        );
        self.phase = Phase::AwaitingReconnect;
        self.generation += 1;
        Ok(())
    }
    pub fn transport_failed(&mut self) -> Result<()> {
        ensure!(
            !matches!(
                self.phase,
                Phase::Completed | Phase::Cancelled | Phase::Review
            ),
            "Task is terminal"
        );
        self.review(if self.activation_intent_recorded {
            "Activation outcome is ambiguous; do not resend, remove the package or assume rollback"
        } else {
            "Stopped before activation; retain candidate and verify target before retry"
        });
        Ok(())
    }
    pub fn reconnected(&mut self, usb_serial: &str) -> Result<()> {
        ensure!(
            self.phase == Phase::AwaitingReconnect,
            "Unexpected reconnect"
        );
        if usb_serial != self.usb_serial {
            self.review("A different device reconnected");
        } else {
            self.phase = Phase::AwaitingReadback;
            self.generation += 1;
        }
        Ok(())
    }
    pub fn readback(&mut self, info: &DeviceInfo) -> Result<()> {
        ensure!(
            self.phase == Phase::AwaitingReadback,
            "Reconnect alone cannot establish success"
        );
        self.observed_readback = Some(info.clone());
        if !same_device(&self.identity, info)
            || info.version != self.target_version
            || Version::parse(&info.revision).ok().as_ref() != Some(&info.version)
        {
            self.review("Installed version or device identity did not match the candidate");
        } else {
            self.phase = Phase::Completed;
            self.generation += 1;
        }
        Ok(())
    }
    pub fn cancel(&mut self) -> Result<()> {
        ensure!(
            !self.activation_intent_recorded
                && matches!(self.phase, Phase::Prepared | Phase::PayloadVerified),
            "Cannot cancel or roll back after activation intent"
        );
        self.phase = Phase::Cancelled;
        self.generation += 1;
        Ok(())
    }
    pub fn checkpoint(&self) -> Result<Vec<u8>> {
        let payload = serde_json::to_vec(self)?;
        Ok(serde_json::to_vec(
            &serde_json::json!({"sha256":format!("{:x}",Sha256::digest(&payload)),"payload":payload}),
        )?)
    }
    pub fn restore(bytes: &[u8]) -> Result<Self> {
        #[derive(Deserialize)]
        struct Envelope {
            sha256: String,
            payload: Vec<u8>,
        }
        ensure!(bytes.len() <= 32768, "Oversized firmware checkpoint");
        let e: Envelope = serde_json::from_slice(bytes)?;
        ensure!(
            format!("{:x}", Sha256::digest(&e.payload)) == e.sha256,
            "Firmware checkpoint checksum mismatch"
        );
        // Deserialize only at this validated checkpoint boundary. Session itself
        // intentionally does not implement Deserialize or expose mutable fields.
        #[derive(Deserialize)]
        struct Record {
            schema: u8,
            phase: Phase,
            generation: u64,
            identity: DeviceInfo,
            usb_serial: String,
            image_sha256: String,
            target_version: Version,
            activation_intent_recorded: bool,
            review_reason: Option<String>,
            observed_readback: Option<DeviceInfo>,
        }
        let r: Record = serde_json::from_slice(&e.payload)?;
        let mut s = Self {
            schema: r.schema,
            phase: r.phase,
            generation: r.generation,
            identity: r.identity,
            usb_serial: r.usb_serial,
            image_sha256: r.image_sha256,
            target_version: r.target_version,
            activation_intent_recorded: r.activation_intent_recorded,
            review_reason: r.review_reason,
            observed_readback: r.observed_readback,
        };
        ensure!(
            s.schema == 1 && s.generation <= 16 && digest_valid(&s.image_sha256),
            "Unsupported firmware checkpoint"
        );
        let baseline = Self::new(
            s.identity.clone(),
            s.usb_serial.clone(),
            s.image_sha256.clone(),
            s.target_version.clone(),
        )?;
        ensure!(
            baseline.schema == s.schema,
            "Invalid firmware checkpoint identity"
        );
        ensure!(
            s.activation_intent_recorded
                || !matches!(
                    s.phase,
                    Phase::ActivationIntent
                        | Phase::AwaitingReconnect
                        | Phase::AwaitingReadback
                        | Phase::Completed
                ),
            "Missing activation intent in checkpoint"
        );
        ensure!(
            !s.activation_intent_recorded
                || !matches!(
                    s.phase,
                    Phase::Prepared | Phase::PayloadVerified | Phase::Cancelled
                ),
            "Inconsistent activation checkpoint"
        );
        if s.phase == Phase::Completed {
            let observed = s
                .observed_readback
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Completed checkpoint has no readback evidence"))?;
            ensure!(
                same_device(&s.identity, observed)
                    && observed.version == s.target_version
                    && Version::parse(&observed.revision)? == observed.version,
                "Invalid completed firmware checkpoint"
            );
        }
        if s.activation_intent_recorded && !matches!(s.phase, Phase::Completed | Phase::Review) {
            s.review("Restart after activation intent requires read-only review; never resend automatically");
        }
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session() -> Session {
        Session::new(
            DeviceInfo {
                vendor: "SONY".into(),
                model: "NW-A918".into(),
                version: Version { major: 1, minor: 2 },
                revision: "1.02".into(),
                subrevision: "00".into(),
                device_serial_hex: "01".repeat(16),
                storage_size_hex: "00000000".into(),
            },
            "usb-a918".into(),
            "ab".repeat(32),
            Version { major: 1, minor: 3 },
        )
        .unwrap()
    }
    fn verified() -> Session {
        let mut s = session();
        s.payload_verified(
            &s.image_sha256.clone(),
            &s.identity.clone(),
            &s.usb_serial.clone(),
        )
        .unwrap();
        s
    }
    #[test]
    fn simulated_success_requires_exact_identity_and_post_reboot_version() {
        let mut s = verified();
        s.record_activation_intent().unwrap();
        s.activation_acknowledged().unwrap();
        s.reconnected("usb-a918").unwrap();
        assert_ne!(s.phase, Phase::Completed);
        let mut updated = s.identity.clone();
        updated.version = s.target_version.clone();
        updated.revision = "1.03".into();
        s.readback(&updated).unwrap();
        assert_eq!(s.phase, Phase::Completed);
        assert_eq!(Session::restore(&s.checkpoint().unwrap()).unwrap(), s);
    }
    #[test]
    fn every_interruption_boundary_is_fail_closed_and_never_resends() {
        let mut stages = vec![session(), verified()];
        let mut s = verified();
        s.record_activation_intent().unwrap();
        stages.push(s.clone());
        s.activation_acknowledged().unwrap();
        stages.push(s.clone());
        s.reconnected("usb-a918").unwrap();
        stages.push(s);
        for mut stage in stages {
            let sent = stage.activation_intent_recorded;
            let mut restored = Session::restore(&stage.checkpoint().unwrap()).unwrap();
            if sent {
                assert_eq!(restored.phase, Phase::Review);
                assert!(restored.record_activation_intent().is_err());
                assert!(restored.cancel().is_err());
            }
            stage.transport_failed().unwrap();
            assert_eq!(stage.phase, Phase::Review);
            assert!(stage.record_activation_intent().is_err());
            assert_eq!(stage.activation_intent_recorded, sent);
        }
    }
    #[test]
    fn changed_payload_wrong_device_and_wrong_version_never_complete() {
        let mut s = session();
        s.payload_verified(&"ff".repeat(32), &s.identity.clone(), "usb-a918")
            .unwrap();
        assert_eq!(s.phase, Phase::Review);
        let mut s = verified();
        s.record_activation_intent().unwrap();
        s.activation_acknowledged().unwrap();
        s.reconnected("other").unwrap();
        assert_eq!(s.phase, Phase::Review);
        for change_serial in [false, true] {
            let mut s = verified();
            s.record_activation_intent().unwrap();
            s.activation_acknowledged().unwrap();
            s.reconnected("usb-a918").unwrap();
            let mut info = s.identity.clone();
            if change_serial {
                info.version = s.target_version.clone();
                info.device_serial_hex = "02".repeat(16);
            }
            s.readback(&info).unwrap();
            assert_eq!(s.phase, Phase::Review);
        }
    }
    #[test]
    fn same_version_downgrade_unknown_identity_and_bad_journals_are_rejected() {
        let s = session();
        assert!(
            Session::new(
                s.identity.clone(),
                s.usb_serial.clone(),
                s.image_sha256.clone(),
                Version { major: 1, minor: 1 }
            )
            .is_err()
        );
        assert!(
            Session::new(
                s.identity.clone(),
                s.usb_serial.clone(),
                s.image_sha256.clone(),
                s.identity.version.clone()
            )
            .is_err()
        );
        let mut identity = s.identity.clone();
        identity.model = "NW-A919".into();
        assert!(
            Session::new(
                identity,
                s.usb_serial.clone(),
                s.image_sha256.clone(),
                s.target_version.clone()
            )
            .is_err()
        );
        let bytes = s.checkpoint().unwrap();
        for len in 0..bytes.len() {
            assert!(Session::restore(&bytes[..len]).is_err());
        }
        let mut e: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        e["sha256"] = "00".repeat(32).into();
        assert!(Session::restore(&serde_json::to_vec(&e).unwrap()).is_err());
    }
    #[test]
    fn cancellation_is_only_pre_activation_and_live_gate_is_always_disabled() {
        for mut s in [session(), verified()] {
            s.cancel().unwrap();
            assert_eq!(s.phase, Phase::Cancelled);
            assert!(s.record_activation_intent().is_err());
        }
        assert_eq!(live_gate()["firmware_write_enabled"], false);
        assert_eq!(live_gate()["raw_vendor_queries_enabled"], false);
    }
    #[test]
    fn known_high_battery_and_identity_do_not_override_unverified_recovery() {
        let s = session();
        for percent in [None, Some(0), Some(20), Some(100)] {
            let power = PowerStatus {
                supply_status_raw: 1,
                battery_status_raw: 8,
                percent,
                life_seconds: None,
                full_life_seconds: None,
            };
            let report = preflight(Some(&s.identity), Some(&power));
            assert_eq!(report["firmware_write_enabled"], false);
            assert!(
                report["blockers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|b| b.as_str().unwrap().contains("recovery"))
            );
        }
    }
    #[test]
    fn completed_checkpoints_require_recorded_readback_evidence() {
        let mut s = verified();
        s.record_activation_intent().unwrap();
        s.phase = Phase::Completed;
        assert!(Session::restore(&s.checkpoint().unwrap()).is_err());
        s.observed_readback = Some(s.identity.clone());
        assert!(Session::restore(&s.checkpoint().unwrap()).is_err());
    }
}
