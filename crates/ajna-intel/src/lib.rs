// ajna-intel — Ajna Intel: device posture and integrity intelligence.
//
// Split of responsibilities:
//   - Platform shells (Swift / Kotlin / JS) gather raw, cheap facts:
//     which artifact paths exist, which libraries are loaded, build
//     properties, debugger state. They make NO trust decisions.
//   - This crate evaluates those facts against known-compromise catalogs
//     (`checks`), produces a risk-scored `PostureReport`, and signs it via
//     the shared ajna-crypto registry so the backend can verify provenance
//     with the same PQC machinery used for IDV scan results.

pub mod checks;

use ajna_crypto::{AjnaSigner, SignerError};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Risk weights and verdict thresholds — product policy, documented here so
/// integrators can reason about scores. Root/jailbreak/hooking findings are
/// verdict-critical: they force `Compromised` regardless of score.
const WEIGHT_ROOT_OR_JAILBREAK: u8 = 60;
const WEIGHT_HOOKING: u8 = 50;
const WEIGHT_DEBUGGER: u8 = 30;
const WEIGHT_EMULATOR: u8 = 25;
const WEIGHT_SELINUX_PERMISSIVE: u8 = 20;
const WEIGHT_DEBUGGABLE_BUILD: u8 = 15;
const SUSPICIOUS_SCORE_THRESHOLD: u8 = 25;
const COMPROMISED_SCORE_THRESHOLD: u8 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Ios,
    Android,
    Web,
}

/// Raw facts gathered by the platform shell. No trust decisions here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceIndicators {
    pub platform: Platform,
    /// Artifact paths the shell confirmed exist on the filesystem.
    #[serde(default)]
    pub present_paths: Vec<String>,
    /// Names of libraries loaded into the host process.
    #[serde(default)]
    pub loaded_libraries: Vec<String>,
    /// Android build properties (empty map on iOS/Web).
    #[serde(default)]
    pub build_properties: BTreeMap<String, String>,
    #[serde(default)]
    pub debugger_attached: bool,
    /// `None` where SELinux does not apply (iOS/Web).
    #[serde(default)]
    pub selinux_enforcing: Option<bool>,
}

impl DeviceIndicators {
    /// A clean baseline for the given platform (useful as a builder start).
    pub fn clean(platform: Platform) -> Self {
        Self {
            platform,
            present_paths: Vec::new(),
            loaded_libraries: Vec::new(),
            build_properties: BTreeMap::new(),
            debugger_attached: false,
            selinux_enforcing: None,
        }
    }
}

/// One concrete reason the device posture is degraded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Finding {
    RootArtifact { path: String },
    JailbreakArtifact { path: String },
    HookingFramework { library: String, marker: String },
    DebuggerAttached,
    EmulatorProperty { key: String, value: String },
    SelinuxPermissive,
    DebuggableBuild,
}

impl Finding {
    fn weight(&self) -> u8 {
        match self {
            Self::RootArtifact { .. } | Self::JailbreakArtifact { .. } => WEIGHT_ROOT_OR_JAILBREAK,
            Self::HookingFramework { .. } => WEIGHT_HOOKING,
            Self::DebuggerAttached => WEIGHT_DEBUGGER,
            Self::EmulatorProperty { .. } => WEIGHT_EMULATOR,
            Self::SelinuxPermissive => WEIGHT_SELINUX_PERMISSIVE,
            Self::DebuggableBuild => WEIGHT_DEBUGGABLE_BUILD,
        }
    }

    /// Findings that force a `Compromised` verdict on their own.
    fn is_critical(&self) -> bool {
        matches!(
            self,
            Self::RootArtifact { .. }
                | Self::JailbreakArtifact { .. }
                | Self::HookingFramework { .. }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Trusted,
    Suspicious,
    Compromised,
}

/// The signed-payload output of a posture evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostureReport {
    pub platform: Platform,
    /// 0–100; sum of finding weights, capped.
    pub risk_score: u8,
    pub verdict: Verdict,
    pub findings: Vec<Finding>,
    /// Unix seconds, supplied by the caller for determinism.
    pub evaluated_at_unix: u64,
}

/// Evaluate raw device facts into a risk-scored posture report.
///
/// Pure function: same indicators + timestamp always produce an identical
/// report (and therefore identical canonical bytes for signing).
pub fn evaluate(indicators: &DeviceIndicators, evaluated_at_unix: u64) -> PostureReport {
    let mut findings = Vec::new();

    for path in &indicators.present_paths {
        if checks::is_known_root_artifact(path) {
            findings.push(Finding::RootArtifact { path: path.clone() });
        } else if checks::is_known_jailbreak_artifact(path) {
            findings.push(Finding::JailbreakArtifact { path: path.clone() });
        }
    }

    for library in &indicators.loaded_libraries {
        if let Some(marker) = checks::hooking_marker_in(library) {
            findings.push(Finding::HookingFramework {
                library: library.clone(),
                marker: marker.to_owned(),
            });
        }
    }

    for (key, value) in &indicators.build_properties {
        if checks::is_emulator_property(key, value) {
            findings.push(Finding::EmulatorProperty { key: key.clone(), value: value.clone() });
        }
    }
    let debuggable = checks::DEBUGGABLE_PROPERTY;
    if indicators.build_properties.get(debuggable.0).map(String::as_str) == Some(debuggable.1) {
        findings.push(Finding::DebuggableBuild);
    }

    if indicators.debugger_attached {
        findings.push(Finding::DebuggerAttached);
    }
    if indicators.selinux_enforcing == Some(false) {
        findings.push(Finding::SelinuxPermissive);
    }

    let risk_score = findings
        .iter()
        .fold(0u16, |acc, f| acc + u16::from(f.weight()))
        .min(100) as u8;

    let verdict = if findings.iter().any(Finding::is_critical)
        || risk_score >= COMPROMISED_SCORE_THRESHOLD
    {
        Verdict::Compromised
    } else if risk_score >= SUSPICIOUS_SCORE_THRESHOLD {
        Verdict::Suspicious
    } else {
        Verdict::Trusted
    };

    PostureReport {
        platform: indicators.platform,
        risk_score,
        verdict,
        findings,
        evaluated_at_unix,
    }
}

/// A posture report plus detached signature and provenance material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedPostureReport {
    /// The exact JSON bytes that were signed.
    pub report_json: String,
    /// ajna-crypto algorithm identifier (e.g. "ed25519", "ml-dsa-65").
    pub algorithm: String,
    pub signature_b64: String,
    pub public_key_b64: String,
}

impl PostureReport {
    /// Deterministic JSON payload: struct field order is fixed and all maps
    /// upstream are BTreeMaps, so serialization is canonical.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("PostureReport serialization cannot fail")
    }

    /// Sign this report with any signer from the shared ajna-crypto registry.
    pub fn sign(&self, signer: &dyn AjnaSigner) -> Result<SignedPostureReport, SignerError> {
        let report_json = self.to_json();
        let signature = signer.sign(report_json.as_bytes())?;
        let b64 = base64::engine::general_purpose::STANDARD;
        Ok(SignedPostureReport {
            report_json,
            algorithm: signer.algorithm_id().to_owned(),
            signature_b64: b64.encode(signature),
            public_key_b64: b64.encode(signer.public_key_bytes()),
        })
    }
}

impl SignedPostureReport {
    /// Verify the detached signature with the signer that produced it
    /// (or any signer holding the same keypair).
    ///
    /// Normalizes the ajna-crypto contract (which reports an unsatisfied
    /// verification equation as `Err(VerificationFailed)`) into a plain
    /// boolean: `Ok(false)` means "signature invalid", errors are reserved
    /// for malformed input.
    pub fn verify_with(&self, signer: &dyn AjnaSigner) -> Result<bool, SignerError> {
        let b64 = base64::engine::general_purpose::STANDARD;
        let Ok(signature) = b64.decode(&self.signature_b64) else {
            return Ok(false);
        };
        match signer.verify(self.report_json.as_bytes(), &signature) {
            Ok(valid) => Ok(valid),
            Err(SignerError::VerificationFailed(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ajna_crypto::EdDsaSigner;

    const NOW: u64 = 1_751_700_000;

    #[test]
    fn clean_device_is_trusted_with_zero_score() {
        let report = evaluate(&DeviceIndicators::clean(Platform::Android), NOW);
        assert_eq!(report.verdict, Verdict::Trusted);
        assert_eq!(report.risk_score, 0);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn su_binary_forces_compromised_verdict() {
        let mut indicators = DeviceIndicators::clean(Platform::Android);
        indicators.present_paths.push("/system/bin/su".to_owned());
        let report = evaluate(&indicators, NOW);
        assert_eq!(report.verdict, Verdict::Compromised);
        assert!(matches!(report.findings[0], Finding::RootArtifact { .. }));
    }

    #[test]
    fn frida_gadget_is_detected_case_insensitively() {
        let mut indicators = DeviceIndicators::clean(Platform::Ios);
        indicators.loaded_libraries.push("FridaGadget.dylib".to_owned());
        let report = evaluate(&indicators, NOW);
        assert_eq!(report.verdict, Verdict::Compromised);
        assert!(matches!(
            &report.findings[0],
            Finding::HookingFramework { marker, .. } if marker == "frida"
        ));
    }

    #[test]
    fn emulator_plus_debugger_is_suspicious_not_compromised() {
        let mut indicators = DeviceIndicators::clean(Platform::Android);
        indicators
            .build_properties
            .insert("ro.kernel.qemu".to_owned(), "1".to_owned());
        indicators.debugger_attached = true;
        let report = evaluate(&indicators, NOW);
        assert_eq!(report.risk_score, WEIGHT_EMULATOR + WEIGHT_DEBUGGER);
        assert_eq!(report.verdict, Verdict::Suspicious);
    }

    #[test]
    fn risk_score_is_capped_at_100() {
        let mut indicators = DeviceIndicators::clean(Platform::Android);
        indicators.present_paths.push("/system/bin/su".to_owned());
        indicators.present_paths.push("/sbin/su".to_owned());
        indicators.loaded_libraries.push("libfrida.so".to_owned());
        let report = evaluate(&indicators, NOW);
        assert_eq!(report.risk_score, 100);
    }

    #[test]
    fn evaluation_is_deterministic() {
        let mut indicators = DeviceIndicators::clean(Platform::Android);
        indicators.present_paths.push("/data/adb/magisk".to_owned());
        let a = evaluate(&indicators, NOW).to_json();
        let b = evaluate(&indicators, NOW).to_json();
        assert_eq!(a, b);
    }

    #[test]
    fn signed_report_verifies_and_tamper_is_rejected() {
        let signer = EdDsaSigner::new();
        let report = evaluate(&DeviceIndicators::clean(Platform::Ios), NOW);
        let mut signed = report.sign(&signer).expect("signing must succeed");

        assert!(signed.verify_with(&signer).expect("verify must run"));

        signed.report_json = signed.report_json.replace("trusted", "compromised");
        assert!(!signed.verify_with(&signer).expect("verify must run"));
    }
}
