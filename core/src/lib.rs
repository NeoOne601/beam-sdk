// core/src/lib.rs
// Ajna SDK core — public API facade.
// Re-exports only. No logic in this file.
//
// Targeting no_std with alloc for cross-platform compatibility:
//   - iOS (arm64): std available, but we stay alloc-compatible.
//   - Android (arm64/armv7): std available.
//   - WASM: no_std with wasm-bindgen allocator.
//
// The `extern crate alloc` import is required for no_std targets.
// On std targets, the alloc crate is re-exported from std and this is a no-op.

// Std is available on all supported targets (iOS arm64, Android arm64/armv7, WASM via wasm32).
// If targeting a bare-metal no_std environment in the future, add `extern crate alloc;
// and replace `std::` imports with `alloc::` imports in the sub-modules.

pub mod crypto;
pub mod ffi;
pub mod field_parser;
pub mod frame;
pub mod pipeline;
pub mod quality;
pub mod result;
pub mod session;
pub mod ui_config;

// ─── Public API surface re-exports ───────────────────────────────────────────

pub use crypto::{CryptoError, MlDsaLevel, MlKemSession, PqcSigner};
pub use ffi::{
    ajna_gate_create, ajna_gate_destroy, ajna_gate_evaluate, ajna_session_create,
    ajna_session_destroy, ajna_session_get_result_json, ajna_session_get_state,
    ajna_session_push_result, ajna_session_start, ajna_ui_config_validate, AjnaGateHandle,
    AjnaSessionHandle, CField, AJNA_ERR_INVALID_CONFIG, AJNA_ERR_INVALID_FRAME,
    AJNA_ERR_NULL_HANDLE, AJNA_ERR_NULL_PTR, AJNA_ERR_OUT_OF_RANGE, AJNA_OK,
};
pub use field_parser::{FieldParser, ParseError, ParsedDocument};
pub use frame::{FrameError, OwnedFrame, PixelFormat, RawFrame};
pub use pipeline::{FramePipeline, PipelineResult};
pub use quality::{Gate, QualityGate, QualityReport};
pub use result::{DocumentField, ScanResult};
pub use session::{ScanSession, SessionConfig, SessionState};
pub use ui_config::{
    OverlayShape, UiAnimations, UiBranding, UiConfig, UiConfigError, UiMode, UiOverlay, UiTheme,
};
