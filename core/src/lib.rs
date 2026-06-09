// core/src/lib.rs
// Beam SDK core — public API facade.
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

pub mod frame;
pub mod quality;
pub mod session;
pub mod result;
pub mod pipeline;
pub mod crypto;
pub mod ffi;
pub mod field_parser;

// ─── Public API surface re-exports ───────────────────────────────────────────

pub use frame::{RawFrame, OwnedFrame, PixelFormat};
pub use quality::{QualityGate, QualityReport, Gate};
pub use session::{ScanSession, SessionConfig, SessionState};
pub use result::{ScanResult, DocumentField};
pub use pipeline::{FramePipeline, PipelineResult};
pub use crypto::{PqcSigner, MlDsaLevel, MlKemSession, CryptoError};
pub use field_parser::{FieldParser, ParsedDocument, ParseError};
pub use ffi::{
    BeamSessionHandle, BeamGateHandle, CField,
    beam_session_create, beam_session_destroy, beam_session_start,
    beam_session_get_state, beam_session_push_result,
    beam_gate_create, beam_gate_evaluate, beam_gate_destroy,
};
