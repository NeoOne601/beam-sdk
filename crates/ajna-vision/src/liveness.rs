// Challenge-response liveness: the session issues an ordered sequence of
// gestures; the platform's face pipeline reports per-frame gesture
// confidences as observations. The FSM enforces order, monotonic timestamps
// (anti-replay), per-challenge attempt budgets, and a wall-clock timeout.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Challenge {
    Blink,
    TurnLeft,
    TurnRight,
    Smile,
    Nod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivenessConfig {
    /// Ordered gesture sequence the user must complete.
    pub challenges: Vec<Challenge>,
    /// Minimum gesture confidence to accept an observation.
    pub min_confidence: f32,
    /// Failed/mismatched observations allowed per challenge before the
    /// session fails.
    pub max_attempts_per_challenge: u32,
    /// Whole-session budget in microseconds, measured from `start`.
    pub session_timeout_us: u64,
}

impl Default for LivenessConfig {
    fn default() -> Self {
        Self {
            challenges: vec![Challenge::Blink, Challenge::TurnLeft],
            min_confidence: 0.85,
            max_attempts_per_challenge: 3,
            session_timeout_us: 30_000_000, // 30 s
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivenessState {
    Idle,
    InProgress { challenge_index: usize },
    Passed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChallengeObservation {
    pub challenge: Challenge,
    /// Gesture confidence reported by the face pipeline, 0.0–1.0.
    pub confidence: f32,
    pub timestamp_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailReason {
    AttemptsExhausted,
    TimedOut,
    NotStarted,
}

/// Outcome of submitting one observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitOutcome {
    /// Challenge cleared; the next one is now current.
    Advanced,
    /// Observation did not clear the challenge; attempts remain.
    Retry { attempts_left: u32 },
    /// All challenges cleared — liveness passed.
    Passed,
    Failed { reason: FailReason },
    /// Observation ignored: non-monotonic timestamp (replay suspect).
    RejectedReplay,
}

#[derive(Debug, Clone)]
pub struct LivenessSession {
    config: LivenessConfig,
    state: LivenessState,
    attempts_used: u32,
    started_at_us: u64,
    last_observation_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivenessConfigError {
    NoChallenges,
    InvalidConfidence,
}

impl LivenessSession {
    pub fn new(config: LivenessConfig) -> Result<Self, LivenessConfigError> {
        if config.challenges.is_empty() {
            return Err(LivenessConfigError::NoChallenges);
        }
        if !(0.0..=1.0).contains(&config.min_confidence) {
            return Err(LivenessConfigError::InvalidConfidence);
        }
        Ok(Self {
            config,
            state: LivenessState::Idle,
            attempts_used: 0,
            started_at_us: 0,
            last_observation_us: 0,
        })
    }

    pub fn start(&mut self, now_us: u64) {
        if self.state == LivenessState::Idle {
            self.state = LivenessState::InProgress { challenge_index: 0 };
            self.started_at_us = now_us;
            self.last_observation_us = now_us;
        }
    }

    pub fn state(&self) -> LivenessState {
        self.state
    }

    /// The gesture the user must currently perform, if the session is live.
    pub fn current_challenge(&self) -> Option<Challenge> {
        match self.state {
            LivenessState::InProgress { challenge_index } => {
                self.config.challenges.get(challenge_index).copied()
            }
            _ => None,
        }
    }

    pub fn submit(&mut self, observation: ChallengeObservation) -> SubmitOutcome {
        let LivenessState::InProgress { challenge_index } = self.state else {
            return SubmitOutcome::Failed { reason: FailReason::NotStarted };
        };

        // Anti-replay: observations must move strictly forward in time.
        if observation.timestamp_us <= self.last_observation_us {
            return SubmitOutcome::RejectedReplay;
        }
        self.last_observation_us = observation.timestamp_us;

        // Session wall-clock budget.
        if observation.timestamp_us >= self.started_at_us + self.config.session_timeout_us {
            self.state = LivenessState::Failed;
            return SubmitOutcome::Failed { reason: FailReason::TimedOut };
        }

        let expected = self.config.challenges[challenge_index];
        let cleared =
            observation.challenge == expected && observation.confidence >= self.config.min_confidence;

        if !cleared {
            self.attempts_used += 1;
            if self.attempts_used >= self.config.max_attempts_per_challenge {
                self.state = LivenessState::Failed;
                return SubmitOutcome::Failed { reason: FailReason::AttemptsExhausted };
            }
            return SubmitOutcome::Retry {
                attempts_left: self.config.max_attempts_per_challenge - self.attempts_used,
            };
        }

        // Challenge cleared: reset the attempt budget and advance.
        self.attempts_used = 0;
        let next = challenge_index + 1;
        if next >= self.config.challenges.len() {
            self.state = LivenessState::Passed;
            SubmitOutcome::Passed
        } else {
            self.state = LivenessState::InProgress { challenge_index: next };
            SubmitOutcome::Advanced
        }
    }

    /// Number of challenges fully cleared so far.
    pub fn challenges_completed(&self) -> usize {
        match self.state {
            LivenessState::Idle => 0,
            LivenessState::InProgress { challenge_index } => challenge_index,
            LivenessState::Passed => self.config.challenges.len(),
            LivenessState::Failed => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(challenge: Challenge, confidence: f32, timestamp_us: u64) -> ChallengeObservation {
        ChallengeObservation { challenge, confidence, timestamp_us }
    }

    fn started_session() -> LivenessSession {
        let mut session = LivenessSession::new(LivenessConfig::default()).unwrap();
        session.start(1_000);
        session
    }

    #[test]
    fn happy_path_clears_both_challenges() {
        let mut session = started_session();
        assert_eq!(session.current_challenge(), Some(Challenge::Blink));

        let outcome = session.submit(observation(Challenge::Blink, 0.95, 2_000));
        assert_eq!(outcome, SubmitOutcome::Advanced);
        assert_eq!(session.current_challenge(), Some(Challenge::TurnLeft));

        let outcome = session.submit(observation(Challenge::TurnLeft, 0.92, 3_000));
        assert_eq!(outcome, SubmitOutcome::Passed);
        assert_eq!(session.state(), LivenessState::Passed);
        assert_eq!(session.challenges_completed(), 2);
    }

    #[test]
    fn wrong_gesture_burns_an_attempt() {
        let mut session = started_session();
        let outcome = session.submit(observation(Challenge::Smile, 0.99, 2_000));
        assert_eq!(outcome, SubmitOutcome::Retry { attempts_left: 2 });
    }

    #[test]
    fn low_confidence_exhausts_attempts_then_fails() {
        let mut session = started_session();
        assert!(matches!(
            session.submit(observation(Challenge::Blink, 0.10, 2_000)),
            SubmitOutcome::Retry { .. }
        ));
        assert!(matches!(
            session.submit(observation(Challenge::Blink, 0.20, 3_000)),
            SubmitOutcome::Retry { .. }
        ));
        assert_eq!(
            session.submit(observation(Challenge::Blink, 0.30, 4_000)),
            SubmitOutcome::Failed { reason: FailReason::AttemptsExhausted }
        );
        assert_eq!(session.state(), LivenessState::Failed);
    }

    #[test]
    fn non_monotonic_timestamp_is_rejected_as_replay() {
        let mut session = started_session();
        session.submit(observation(Challenge::Blink, 0.95, 5_000));
        let outcome = session.submit(observation(Challenge::TurnLeft, 0.95, 5_000));
        assert_eq!(outcome, SubmitOutcome::RejectedReplay);
        // Session unaffected by the replayed frame.
        assert_eq!(session.current_challenge(), Some(Challenge::TurnLeft));
    }

    #[test]
    fn session_times_out_on_wall_clock_budget() {
        let mut session = started_session();
        let late = 1_000 + LivenessConfig::default().session_timeout_us + 1;
        assert_eq!(
            session.submit(observation(Challenge::Blink, 0.95, late)),
            SubmitOutcome::Failed { reason: FailReason::TimedOut }
        );
    }

    #[test]
    fn submit_before_start_fails() {
        let mut session = LivenessSession::new(LivenessConfig::default()).unwrap();
        assert_eq!(
            session.submit(observation(Challenge::Blink, 0.95, 2_000)),
            SubmitOutcome::Failed { reason: FailReason::NotStarted }
        );
    }

    #[test]
    fn empty_challenge_list_is_rejected() {
        let config = LivenessConfig { challenges: vec![], ..Default::default() };
        assert_eq!(
            LivenessSession::new(config).unwrap_err(),
            LivenessConfigError::NoChallenges
        );
    }
}
