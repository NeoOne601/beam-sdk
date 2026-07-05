// ajna-idv — Ajna IDV product facade.
//
// Layers a product-shaped API over the ajna-core scanning engine:
//   - `IdvSession`: capture session bound to a declarative `UiConfig`
//     (stock, custom-branded, or headless capture UI).
//   - `HeadlessScanner`: fully safe frame-feeding API for hosts that build
//     their own capture experience end to end.
//   - Signer bootstrap: registers the shared ajna-crypto algorithms
//     (Ed25519 + ML-DSA-65) so results carry PQC provenance and the backend
//     algorithm negotiation (`POST /v1/session/init`) has real capabilities
//     to advertise.
//
// No scanning logic lives here — quality gates, session state, and result
// signing stay in ajna-core. This crate decides *product* shape only.

mod headless;

pub use headless::{HeadlessError, HeadlessScanner};

// Re-exported engine surface so integrators depend on ajna-idv alone.
pub use ajna_core::{
    DocumentField, FrameError, Gate, OwnedFrame, PipelineResult, PixelFormat, QualityReport,
    RawFrame, ScanResult, SessionConfig, SessionState, UiConfig, UiConfigError, UiMode,
};

use ajna_core::FramePipeline;

/// Everything needed to open an IDV capture session.
#[derive(Default)]
pub struct IdvConfig {
    pub session: SessionConfig,
    pub ui: UiConfig,
}

/// An identity-document capture session with its UI contract attached.
///
/// The engine below is UI-agnostic; `ui_config()` is what the platform shell
/// (Swift / Kotlin / TS) reads to decide what — if anything — to render.
pub struct IdvSession {
    pipeline: FramePipeline,
    ui: UiConfig,
}

impl IdvSession {
    /// Validates the UI configuration and opens the session.
    pub fn new(config: IdvConfig) -> Result<Self, UiConfigError> {
        config.ui.validate()?;
        Ok(Self {
            pipeline: FramePipeline::new(config.session),
            ui: config.ui,
        })
    }

    /// The declarative UI contract for this session.
    pub fn ui_config(&self) -> &UiConfig {
        &self.ui
    }

    pub fn state(&self) -> SessionState {
        self.pipeline.session_state()
    }

    /// Process one camera frame.
    ///
    /// # Safety
    /// Same contract as [`ajna_core::FramePipeline::process_frame`]:
    /// `frame.y_plane` must be valid for `frame.width * frame.height` bytes
    /// for the duration of the call.
    pub unsafe fn process_frame(&mut self, frame: &RawFrame, now_us: u64) -> PipelineResult {
        self.pipeline.process_frame(frame, now_us)
    }

    /// Push an inference result (host-run or bridge-run model output),
    /// transitioning the session to `Complete`.
    pub fn push_result(&mut self, result: ScanResult) {
        self.pipeline.push_result(result);
    }
}

/// Register the shared ajna-crypto signers (Ed25519 first for compatibility,
/// then ML-DSA-65 for post-quantum provenance) into the global registry.
///
/// Idempotent: an already-populated registry is left untouched, so hosts and
/// tests may call this freely at startup.
pub fn init_signers() {
    let registry = ajna_crypto::global_registry();
    let mut guard = registry
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if guard.supported_algorithms().is_empty() {
        guard.register(ajna_crypto::EdDsaSigner::new());
        guard.register(ajna_crypto::MlDsaSigner::new());
    }
}

/// Algorithm identifiers this SDK build can sign with, in preference order.
/// Feeds the backend's `POST /v1/session/init` negotiation.
pub fn available_algorithms() -> Vec<String> {
    init_signers();
    ajna_crypto::global_registry()
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .supported_algorithms()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_rejects_invalid_ui_config() {
        let config = IdvConfig {
            session: SessionConfig::default(),
            ui: UiConfig {
                theme: ajna_core::UiTheme {
                    primary_color: "not-a-color".to_owned(),
                    ..Default::default()
                },
                ..Default::default()
            },
        };
        assert!(IdvSession::new(config).is_err());
    }

    #[test]
    fn new_session_starts_scanning_with_ui_contract() {
        let session = IdvSession::new(IdvConfig::default()).expect("default config is valid");
        assert_eq!(session.state(), SessionState::Scanning);
        assert_eq!(session.ui_config().mode, UiMode::Default);
    }

    #[test]
    fn signer_bootstrap_exposes_classical_and_pqc_algorithms() {
        let algos = available_algorithms();
        assert!(
            algos.iter().any(|a| a.contains("ed25519")),
            "expected an Ed25519 signer, got {algos:?}"
        );
        assert!(
            algos.iter().any(|a| a.contains("ml-dsa")),
            "expected an ML-DSA signer, got {algos:?}"
        );
    }
}
