// tests/session_state_tests.rs
// Session state machine tests.
// Tests cover the full lifecycle: Idle → Scanning → Inferring → Complete/Failed.

#[cfg(test)]
mod tests {
    use ajna_core::result::ScanResult;
    use ajna_core::session::{ScanSession, SessionConfig, SessionState};

    fn default_result() -> ScanResult {
        ScanResult {
            fields: vec![],
            raw_mrz: None,
            document_type: "passport".to_string(),
            issuing_country: "USA".to_string(),
            confidence: 0.99,
            pqc_signature: vec![],
            pqc_public_key: vec![],
            nonce: None,
            session_id: None,
            timestamp_iso: None,
            algo: String::new(),
            ajna_version: String::from("2.0"),
            public_key: String::new(),
            jws_token: None,
        }
    }

    // ─── Initial state ────────────────────────────────────────────────────────

    #[test]
    fn new_session_is_idle() {
        let session = ScanSession::new(SessionConfig::default());
        assert_eq!(session.state, SessionState::Idle);
    }

    // ─── start() transitions to Scanning ─────────────────────────────────────

    #[test]
    fn start_transitions_to_scanning() {
        let mut session = ScanSession::new(SessionConfig::default());
        session.start(0);
        assert_eq!(session.state, SessionState::Scanning);
    }

    // ─── record_quality_frame() returns true on Nth call ──────────────────────

    #[test]
    fn third_quality_frame_triggers_inference() {
        let mut session = ScanSession::new(SessionConfig {
            min_quality_frames: 3,
            ..SessionConfig::default()
        });
        session.start(0);
        assert!(
            !session.record_quality_frame(),
            "1st frame should not trigger"
        );
        assert!(
            !session.record_quality_frame(),
            "2nd frame should not trigger"
        );
        assert!(
            session.record_quality_frame(),
            "3rd frame must trigger inference"
        );
        assert_eq!(session.state, SessionState::Inferring);
    }

    // ─── record_gate_fail() returns true on 60th call ────────────────────────

    #[test]
    fn sixtieth_gate_fail_triggers_relaxation() {
        let mut session = ScanSession::new(SessionConfig {
            adaptive_gate_limit: 60,
            ..SessionConfig::default()
        });
        session.start(0);
        for i in 1..60 {
            assert!(
                !session.record_gate_fail(),
                "fail #{} must not trigger relaxation",
                i
            );
        }
        assert!(
            session.record_gate_fail(),
            "60th consecutive fail must trigger adaptive relaxation"
        );
    }

    // ─── is_timed_out() — within timeout ─────────────────────────────────────

    #[test]
    fn not_timed_out_before_timeout() {
        let config = SessionConfig {
            timeout_ms: 30_000,
            ..SessionConfig::default()
        };
        let mut session = ScanSession::new(config);
        session.start(0);
        // 29 seconds < 30 second timeout
        assert!(!session.is_timed_out(29_000_000));
    }

    // ─── is_timed_out() — at timeout boundary ────────────────────────────────

    #[test]
    fn timed_out_at_or_after_timeout() {
        let config = SessionConfig {
            timeout_ms: 30_000,
            ..SessionConfig::default()
        };
        let mut session = ScanSession::new(config);
        session.start(0);
        // Exactly at timeout
        assert!(session.is_timed_out(30_000_000));
        // Past timeout
        assert!(session.is_timed_out(60_000_000));
    }

    // ─── complete() → state == Complete, result is Some ──────────────────────

    #[test]
    fn complete_sets_state_and_result() {
        let mut session = ScanSession::new(SessionConfig::default());
        session.start(0);
        session.complete(default_result());
        assert_eq!(session.state, SessionState::Complete);
        assert!(session.result.is_some());
    }

    // ─── fail() → state == Failed ─────────────────────────────────────────────

    #[test]
    fn fail_sets_state_to_failed() {
        let mut session = ScanSession::new(SessionConfig::default());
        session.start(0);
        session.fail(Some("test failure"));
        assert_eq!(session.state, SessionState::Failed);
    }

    // ─── record_quality_frame() on Complete session is a no-op ───────────────

    #[test]
    fn quality_frame_on_complete_session_is_noop() {
        let mut session = ScanSession::new(SessionConfig::default());
        session.start(0);
        session.complete(default_result());
        assert_eq!(session.state, SessionState::Complete);
        // This must not panic, must not change state, must return false
        let changed = session.record_quality_frame();
        assert!(
            !changed,
            "record_quality_frame on Complete must return false"
        );
        assert_eq!(
            session.state,
            SessionState::Complete,
            "state must remain Complete after no-op"
        );
    }
}
