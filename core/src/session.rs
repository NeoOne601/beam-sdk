// core/src/session.rs
// Scan session state machine.
// Tracks quality frame accumulation, gate failure counts, adaptive relaxation,
// timeout, and final result storage.

use crate::result::ScanResult;

/// Session lifecycle states. repr(C) for FFI readability.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Initial state. No scan in progress.
    Idle = 0,
    /// Camera running, quality gates evaluating frames.
    Scanning = 1,
    /// Sufficient quality frames accumulated. C++ inference in progress.
    Inferring = 2,
    /// Inference complete. ScanResult available.
    Complete = 3,
    /// Session ended in failure (timeout, repeated gate failure, inference error).
    Failed = 4,
}

/// Session configuration. Mirrors AjnaScanConfig on iOS/Android.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Number of quality-passing frames required before inference is triggered.
    pub min_quality_frames: u32,
    /// Session timeout in milliseconds. Default: 30000 (30 seconds).
    pub timeout_ms: u64,
    /// Consecutive gate failures that trigger adaptive relaxation.
    /// Default: 60 (≈ 2.4 seconds at 25fps).
    pub adaptive_gate_limit: u32,
    /// Whether to PQC-sign the result with ML-DSA Level3.
    /// Set to false in dev builds to reduce overhead.
    pub pqc_sign_result: bool,
    /// Whether to include the raw MRZ string in ScanResult.
    pub include_raw_mrz: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            min_quality_frames: 3,
            timeout_ms: 30_000,
            adaptive_gate_limit: 60,
            pqc_sign_result: true,
            include_raw_mrz: false,
        }
    }
}

/// The scan session state machine.
pub struct ScanSession {
    pub config: SessionConfig,
    pub state: SessionState,
    /// Number of frames that passed all quality gates this session.
    pub quality_frame_count: u32,
    /// Consecutive frames that failed any quality gate.
    pub consecutive_gate_fails: u32,
    /// Monotonic timestamp (µs) when start() was called.
    pub start_timestamp_us: u64,
    /// Final scan result, populated by complete().
    pub result: Option<ScanResult>,
}

impl ScanSession {
    /// Create a new session in Idle state.
    pub fn new(config: SessionConfig) -> Self {
        Self {
            config,
            state: SessionState::Idle,
            quality_frame_count: 0,
            consecutive_gate_fails: 0,
            start_timestamp_us: 0,
            result: None,
        }
    }

    /// Transition to Scanning state and record the start timestamp.
    pub fn start(&mut self, timestamp_us: u64) {
        self.state = SessionState::Scanning;
        self.start_timestamp_us = timestamp_us;
    }

    /// Record a frame that passed all quality gates.
    ///
    /// Returns `true` when `min_quality_frames` threshold is reached,
    /// signalling that inference should be triggered.
    /// After the session reaches Complete or Failed, this is a no-op.
    pub fn record_quality_frame(&mut self) -> bool {
        if self.state != SessionState::Scanning {
            return false;
        }
        self.consecutive_gate_fails = 0;
        self.quality_frame_count += 1;
        if self.quality_frame_count >= self.config.min_quality_frames {
            self.state = SessionState::Inferring;
            return true;
        }
        false
    }

    /// Record a frame that failed at least one quality gate.
    ///
    /// Returns `true` when `adaptive_gate_limit` consecutive failures are reached,
    /// signalling that the caller should apply adaptive gate relaxation.
    pub fn record_gate_fail(&mut self) -> bool {
        if self.state != SessionState::Scanning {
            return false;
        }
        self.consecutive_gate_fails += 1;
        self.consecutive_gate_fails >= self.config.adaptive_gate_limit
    }

    /// Returns `true` if `now_us` has exceeded start + timeout_ms.
    pub fn is_timed_out(&self, now_us: u64) -> bool {
        let timeout_us = self.config.timeout_ms * 1_000;
        now_us >= self.start_timestamp_us.saturating_add(timeout_us)
    }

    /// Transition from Scanning to Inferring manually (e.g. after adaptive relaxation accepts).
    pub fn set_inferring(&mut self) {
        if self.state == SessionState::Scanning {
            self.state = SessionState::Inferring;
        }
    }

    /// Mark the session complete with the given ScanResult.
    pub fn complete(&mut self, result: ScanResult) {
        self.result = Some(result);
        self.state = SessionState::Complete;
    }

    /// Mark the session as failed with an optional reason string.
    pub fn fail(&mut self, _reason: Option<&str>) {
        self.state = SessionState::Failed;
    }
}
