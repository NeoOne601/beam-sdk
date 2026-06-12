// core/src/pipeline.rs
// FramePipeline — the top-level orchestrator.
// Routes frames through quality gates, manages adaptive relaxation,
// and coordinates the session state machine.
//
// CRITICAL INVARIANT (enforced here):
//   The C++/GPU inference layer is NEVER invoked on a frame that did not
//   reach Gate::Accepted. This file is the sole place where AcceptedForInference
//   is returned. Tests must assert this invariant explicitly.

use crate::frame::RawFrame;
use crate::quality::{Gate, QualityGate, QualityReport};
use crate::result::ScanResult;
use crate::session::{ScanSession, SessionConfig};

/// Output of a single pipeline tick.
#[derive(Debug)]
pub enum PipelineResult {
    /// Frame rejected by a quality gate. Contains the gate report for diagnostics.
    Rejected(QualityReport),
    /// Frame passed all gates. Caller must forward to C++ inference.
    /// The session has transitioned to Inferring.
    AcceptedForInference,
    /// Inference complete. Session is in Complete state.
    Complete(ScanResult),
}

/// Top-level frame processing pipeline.
pub struct FramePipeline {
    gate: QualityGate,
    session: ScanSession,
}

impl FramePipeline {
    /// Create a new pipeline with the given session configuration.
    pub fn new(config: SessionConfig) -> Self {
        let mut session = ScanSession::new(config);
        session.start(0); // Start immediately — caller provides real timestamps via process_frame
        Self {
            gate: QualityGate::default(),
            session,
        }
    }

    /// Process one frame through the quality gate pipeline.
    ///
    /// Gate ordering (non-negotiable per architectural constraints):
    ///   1. Timeout check
    ///   2. Blur
    ///   3. Exposure
    ///   4. Motion
    ///   5. Boundary
    ///
    /// Adaptive relaxation is applied when `adaptive_gate_limit` consecutive failures occur.
    /// Relaxation: blur_threshold -15%, exposure range +15%, motion_threshold +15%.
    ///
    /// # Safety
    /// `frame.y_plane` must be valid for `frame.width * frame.height` bytes.
    pub unsafe fn process_frame(&mut self, frame: &RawFrame, now_us: u64) -> PipelineResult {
        // If already complete, return the stored result
        if let crate::session::SessionState::Complete = self.session.state {
            if let Some(result) = self.session.result.clone() {
                return PipelineResult::Complete(result);
            }
        }

        // 1. Timeout check — reject before running any gate logic
        if self.session.is_timed_out(now_us) {
            self.session.fail(Some("timeout"));
            let report = QualityReport {
                gate_reached: Gate::Received,
                blur_score: 0.0,
                mean_luma: 0.0,
                p95_luma: 0.0,
                motion_score: 0.0,
                edge_density: 0.0,
            };
            return PipelineResult::Rejected(report);
        }

        // 2–5. Run quality gates in order
        let report = self.gate.evaluate(frame);

        if report.gate_reached != Gate::Accepted {
            // Frame rejected — record gate failure, possibly trigger adaptive relaxation
            let should_relax = self.session.record_gate_fail();
            if should_relax {
                self.apply_adaptive_relaxation();
            }
            return PipelineResult::Rejected(report);
        }

        // Frame accepted — advance the session
        let inference_ready = self.session.record_quality_frame();
        if inference_ready {
            // INVARIANT: only AcceptedForInference is returned when gate_reached == Accepted
            return PipelineResult::AcceptedForInference;
        }

        // Need more quality frames before triggering inference
        PipelineResult::Rejected(report)
    }

    /// Apply adaptive gate relaxation when too many consecutive frames are rejected.
    /// Relaxation schedule (applied once per trigger, cumulative):
    ///   - blur_threshold   ← × 0.85  (accept blurrier frames)
    ///   - min_luma         ← × 0.85  (accept darker frames)
    ///   - max_luma         ← × 1.15  (accept brighter frames)
    ///   - p95_luma_max     ← × 1.15
    ///   - motion_threshold ← × 1.15  (accept more camera shake)
    fn apply_adaptive_relaxation(&mut self) {
        self.gate.blur_threshold *= 0.85;
        self.gate.min_luma *= 0.85;
        self.gate.max_luma *= 1.15;
        self.gate.p95_luma_max *= 1.15;
        self.gate.motion_threshold *= 1.15;
        // edge_min is not relaxed — a document must always be present
    }

    /// Expose session state for external inspection (e.g. UI progress).
    pub fn session_state(&self) -> crate::session::SessionState {
        self.session.state
    }

    /// Push an inference result into the session, transitioning to Complete.
    /// Called by the C++ layer via beam_session_push_result → ffi.rs → here.
    pub fn push_result(&mut self, result: ScanResult) {
        self.session.complete(result);
    }
}
