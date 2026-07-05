// Headless capture: the host owns the camera and the UI entirely, and feeds
// frames to the engine as plain RGBA buffers. This is the "total control"
// integration path — no Ajna-rendered UI, no platform shell required.

use crate::{IdvConfig, IdvSession, PipelineResult, ScanResult, SessionConfig, SessionState};
use ajna_core::{FrameError, OwnedFrame, UiConfig};
use std::fmt;

/// Errors surfaced by the safe headless frame API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadlessError {
    /// `rgba.len()` is smaller than `width * height * 4`.
    BufferTooSmall { expected: usize, actual: usize },
    /// Frame dimensions failed engine validation (see [`FrameError`]).
    Frame(FrameError),
}

impl fmt::Display for HeadlessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooSmall { expected, actual } => {
                write!(f, "RGBA buffer too small: expected {expected} bytes, got {actual}")
            }
            Self::Frame(e) => write!(f, "invalid frame: {e:?}"),
        }
    }
}

impl std::error::Error for HeadlessError {}

/// A fully safe, UI-free scanning session.
///
/// Frames arrive as RGBA byte slices (any resolution ≥ 64×64); the scanner
/// converts to NV12 internally and drives the exact same quality-gate
/// pipeline and PQC signing path as the camera-attached SDK. Hosts run their
/// own inference and deliver output via [`complete_with`](Self::complete_with).
pub struct HeadlessScanner {
    session: IdvSession,
}

impl HeadlessScanner {
    /// Open a headless session. The UI configuration is forced to
    /// [`UiConfig::headless()`] — by construction nothing will render.
    pub fn new(session_config: SessionConfig) -> Self {
        let config = IdvConfig {
            session: session_config,
            ui: UiConfig::headless(),
        };
        Self {
            // Headless UiConfig is a constant known-valid configuration.
            session: IdvSession::new(config).expect("headless UiConfig is always valid"),
        }
    }

    /// Feed one RGBA frame. Returns the pipeline verdict for this frame.
    ///
    /// The buffer is validated *before* any conversion: a short slice is an
    /// error, never a panic or out-of-bounds read.
    pub fn push_rgba_frame(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        timestamp_us: u64,
    ) -> Result<PipelineResult, HeadlessError> {
        let expected = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        if rgba.len() < expected || expected == 0 {
            return Err(HeadlessError::BufferTooSmall { expected, actual: rgba.len() });
        }

        let owned = OwnedFrame::from_rgba(rgba, width, height, timestamp_us);
        let raw = owned.as_raw();
        raw.validate().map_err(HeadlessError::Frame)?;

        // SAFETY: `raw` borrows from `owned`, which lives until the end of
        // this scope; dimensions were validated above.
        Ok(unsafe { self.session.process_frame(&raw, timestamp_us) })
    }

    pub fn state(&self) -> SessionState {
        self.session.state()
    }

    /// Deliver the host-run inference output, completing the session with a
    /// signed result.
    pub fn complete_with(&mut self, result: ScanResult) {
        self.session.push_result(result);
    }

    /// The (headless) UI contract — always `UiMode::Headless`.
    pub fn ui_config(&self) -> &UiConfig {
        self.session.ui_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ajna_core::UiMode;

    /// A uniform mid-gray frame: passes exposure but is rejected by the blur
    /// gate (zero Laplacian variance) — a deterministic pipeline outcome.
    fn gray_frame(width: u32, height: u32) -> Vec<u8> {
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..(width * height) {
            rgba.extend_from_slice(&[128, 128, 128, 255]);
        }
        rgba
    }

    #[test]
    fn headless_session_never_renders_ui() {
        let scanner = HeadlessScanner::new(SessionConfig::default());
        assert_eq!(scanner.ui_config().mode, UiMode::Headless);
        assert!(scanner.ui_config().is_headless());
    }

    #[test]
    fn short_buffer_is_an_error_not_a_panic() {
        let mut scanner = HeadlessScanner::new(SessionConfig::default());
        let result = scanner.push_rgba_frame(&[0u8; 16], 128, 128, 1);
        assert_eq!(
            result.unwrap_err(),
            HeadlessError::BufferTooSmall { expected: 128 * 128 * 4, actual: 16 }
        );
    }

    #[test]
    fn undersized_frame_fails_engine_validation() {
        let mut scanner = HeadlessScanner::new(SessionConfig::default());
        let rgba = gray_frame(32, 32);
        let result = scanner.push_rgba_frame(&rgba, 32, 32, 1);
        assert!(matches!(result, Err(HeadlessError::Frame(_))));
    }

    #[test]
    fn flat_gray_frame_is_gated_and_session_keeps_scanning() {
        let mut scanner = HeadlessScanner::new(SessionConfig::default());
        let rgba = gray_frame(128, 128);
        let verdict = scanner
            .push_rgba_frame(&rgba, 128, 128, 1_000)
            .expect("well-formed frame must be processed");
        assert!(matches!(verdict, PipelineResult::Rejected(_)));
        assert_eq!(scanner.state(), SessionState::Scanning);
    }

    #[test]
    fn host_inference_result_completes_the_session() {
        let mut scanner = HeadlessScanner::new(SessionConfig::default());
        scanner.complete_with(ScanResult::default());
        assert_eq!(scanner.state(), SessionState::Complete);
    }
}
