// Landmark → gesture derivation for the liveness FSM.
//
// MediaPipe FaceMesh (on-device, $0) emits 468 3-D face landmarks per frame.
// The native bridge fills the `FaceLandmarks` seam; this module owns the
// portable, testable geometry that turns a landmark frame into a
// `ChallengeObservation` the existing liveness state machine already consumes:
//   - blink       ← eye aspect ratio (EAR) dropping below a threshold
//   - turn L/R    ← signed yaw estimated from eye-to-nose horizontal offset
//   - smile       ← mouth aspect ratio (MAR) widening past a threshold
//
// Only the landmark indices we use are named; the native side passes the full
// 468-point array and we read the canonical MediaPipe indices out of it.

use crate::liveness::{Challenge, ChallengeObservation};

/// A single 3-D landmark in normalized image coordinates (0.0–1.0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Point3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

/// One frame of MediaPipe FaceMesh output: exactly 468 landmarks.
#[derive(Debug, Clone)]
pub struct FaceLandmarks {
    points: Vec<Point3>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandmarkError {
    /// FaceMesh returns 468 points; anything else is a malformed frame.
    WrongPointCount,
}

// Canonical MediaPipe FaceMesh indices (subset we need).
// Left eye (subject's left / image right) vertical + horizontal corners:
const LEFT_EYE_TOP: usize = 386;
const LEFT_EYE_BOTTOM: usize = 374;
const LEFT_EYE_OUTER: usize = 263;
const LEFT_EYE_INNER: usize = 362;
// Right eye:
const RIGHT_EYE_TOP: usize = 159;
const RIGHT_EYE_BOTTOM: usize = 145;
const RIGHT_EYE_OUTER: usize = 33;
const RIGHT_EYE_INNER: usize = 133;
// Mouth corners + lips:
const MOUTH_LEFT: usize = 291;
const MOUTH_RIGHT: usize = 61;
const MOUTH_TOP: usize = 13;
const MOUTH_BOTTOM: usize = 14;
// Nose tip and cheeks for yaw:
const NOSE_TIP: usize = 1;
const CHEEK_LEFT: usize = 234;
const CHEEK_RIGHT: usize = 454;

/// Thresholds tuned for MediaPipe's normalized coordinate space. Exposed so an
/// integrator can recalibrate per camera/FOV without touching the geometry.
#[derive(Debug, Clone, Copy)]
pub struct GestureThresholds {
    /// EAR below this ⇒ eyes closed (blink).
    pub blink_ear_max: f32,
    /// MAR above this ⇒ smiling.
    pub smile_mar_min: f32,
    /// |yaw ratio| above this ⇒ head turned; sign picks the direction.
    pub turn_yaw_min: f32,
}

impl Default for GestureThresholds {
    fn default() -> Self {
        Self {
            blink_ear_max: 0.18,
            smile_mar_min: 0.55,
            turn_yaw_min: 0.15,
        }
    }
}

impl FaceLandmarks {
    /// Wrap a 468-point frame. Rejects any other length.
    pub fn new(points: Vec<Point3>) -> Result<Self, LandmarkError> {
        if points.len() != 468 {
            return Err(LandmarkError::WrongPointCount);
        }
        Ok(Self { points })
    }

    fn p(&self, i: usize) -> Point3 {
        self.points[i]
    }

    /// Eye aspect ratio averaged across both eyes. Low when eyes are closed.
    pub fn eye_aspect_ratio(&self) -> f32 {
        let left = ear(
            self.p(LEFT_EYE_TOP),
            self.p(LEFT_EYE_BOTTOM),
            self.p(LEFT_EYE_INNER),
            self.p(LEFT_EYE_OUTER),
        );
        let right = ear(
            self.p(RIGHT_EYE_TOP),
            self.p(RIGHT_EYE_BOTTOM),
            self.p(RIGHT_EYE_INNER),
            self.p(RIGHT_EYE_OUTER),
        );
        (left + right) / 2.0
    }

    /// Mouth aspect ratio: width / height. High when smiling (wide) — we use
    /// width-over-height so a grin (wide, not tall) scores above a neutral mouth.
    pub fn mouth_aspect_ratio(&self) -> f32 {
        let width = horiz_dist(self.p(MOUTH_LEFT), self.p(MOUTH_RIGHT));
        let height = vert_dist(self.p(MOUTH_TOP), self.p(MOUTH_BOTTOM)).max(1e-4);
        width / height / 5.0 // /5 normalizes into a ~0..1 band for typical faces
    }

    /// Signed yaw estimate in [-1, 1]. Positive ⇒ subject turned to their right
    /// (image left); negative ⇒ their left. Uses nose-tip offset between cheeks.
    pub fn yaw_ratio(&self) -> f32 {
        let left = self.p(CHEEK_LEFT);
        let right = self.p(CHEEK_RIGHT);
        let nose = self.p(NOSE_TIP);
        let span = (right.x - left.x).abs().max(1e-4);
        let midpoint = (left.x + right.x) / 2.0;
        // Nose right of center ⇒ face turned left, and vice versa. Scale to span.
        ((nose.x - midpoint) / span) * 2.0
    }

    /// Derive the gesture observation for `expected`, given a per-frame
    /// timestamp. Confidence reflects how decisively the threshold was crossed.
    pub fn observe(
        &self,
        expected: Challenge,
        timestamp_us: u64,
        thresholds: GestureThresholds,
    ) -> ChallengeObservation {
        let (detected, confidence) = match expected {
            Challenge::Blink => {
                let ear = self.eye_aspect_ratio();
                (
                    ear <= thresholds.blink_ear_max,
                    ratio_conf(thresholds.blink_ear_max, ear, true),
                )
            }
            Challenge::Smile => {
                let mar = self.mouth_aspect_ratio();
                (
                    mar >= thresholds.smile_mar_min,
                    ratio_conf(thresholds.smile_mar_min, mar, false),
                )
            }
            Challenge::TurnLeft => {
                let yaw = self.yaw_ratio();
                (
                    yaw <= -thresholds.turn_yaw_min,
                    ratio_conf(thresholds.turn_yaw_min, -yaw, false),
                )
            }
            Challenge::TurnRight => {
                let yaw = self.yaw_ratio();
                (
                    yaw >= thresholds.turn_yaw_min,
                    ratio_conf(thresholds.turn_yaw_min, yaw, false),
                )
            }
            Challenge::Nod => {
                // Nod is vertical; approximate with eye-line drop is unreliable,
                // so report low confidence — a nod-capable native path can override.
                (false, 0.0)
            }
        };
        ChallengeObservation {
            challenge: expected,
            // Confidence is 0 when the gesture was not detected, so the FSM's
            // min_confidence gate naturally rejects a non-matching frame.
            confidence: if detected { confidence } else { 0.0 },
            timestamp_us,
        }
    }
}

fn ear(top: Point3, bottom: Point3, inner: Point3, outer: Point3) -> f32 {
    let vertical = vert_dist(top, bottom);
    let horizontal = horiz_dist(inner, outer).max(1e-4);
    vertical / horizontal
}

fn vert_dist(a: Point3, b: Point3) -> f32 {
    (a.y - b.y).abs()
}

fn horiz_dist(a: Point3, b: Point3) -> f32 {
    (a.x - b.x).abs()
}

/// Map how far a measured value cleared its threshold into a 0.5–1.0
/// confidence. `below` = true means "detected when value <= threshold".
///
/// The margin is normalized by the threshold, so "how decisively" is relative
/// to the gesture's own scale: exactly at the threshold ⇒ 0.5, roughly one
/// threshold-width past it ⇒ saturates to 1.0. This keeps a decisive gesture
/// comfortably above the liveness FSM's default 0.85 confidence gate.
fn ratio_conf(threshold: f32, value: f32, below: bool) -> f32 {
    let margin = if below {
        threshold - value
    } else {
        value - threshold
    };
    let normalized = (margin / threshold.max(1e-4)).max(0.0);
    (0.5 + normalized * 0.5).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::liveness::{Challenge, LivenessConfig, LivenessSession, SubmitOutcome};

    /// Build a neutral, eyes-open, forward-facing face, then let tests perturb
    /// specific landmarks to synthesize a gesture.
    fn neutral_face() -> Vec<Point3> {
        let mut pts = vec![Point3::new(0.5, 0.5, 0.0); 468];
        // Eyes open: ~0.03 vertical separation, ~0.12 horizontal.
        pts[LEFT_EYE_TOP] = Point3::new(0.62, 0.40, 0.0);
        pts[LEFT_EYE_BOTTOM] = Point3::new(0.62, 0.43, 0.0);
        pts[LEFT_EYE_INNER] = Point3::new(0.56, 0.415, 0.0);
        pts[LEFT_EYE_OUTER] = Point3::new(0.68, 0.415, 0.0);
        pts[RIGHT_EYE_TOP] = Point3::new(0.38, 0.40, 0.0);
        pts[RIGHT_EYE_BOTTOM] = Point3::new(0.38, 0.43, 0.0);
        pts[RIGHT_EYE_INNER] = Point3::new(0.44, 0.415, 0.0);
        pts[RIGHT_EYE_OUTER] = Point3::new(0.32, 0.415, 0.0);
        // Mouth: closed, moderate width.
        pts[MOUTH_LEFT] = Point3::new(0.60, 0.70, 0.0);
        pts[MOUTH_RIGHT] = Point3::new(0.40, 0.70, 0.0);
        pts[MOUTH_TOP] = Point3::new(0.50, 0.68, 0.0);
        pts[MOUTH_BOTTOM] = Point3::new(0.50, 0.72, 0.0);
        // Cheeks + centered nose (no yaw).
        pts[CHEEK_LEFT] = Point3::new(0.25, 0.5, 0.0);
        pts[CHEEK_RIGHT] = Point3::new(0.75, 0.5, 0.0);
        pts[NOSE_TIP] = Point3::new(0.50, 0.55, 0.0);
        pts
    }

    #[test]
    fn wrong_point_count_is_rejected() {
        assert_eq!(
            FaceLandmarks::new(vec![]).unwrap_err(),
            LandmarkError::WrongPointCount
        );
    }

    #[test]
    fn open_eyes_do_not_register_a_blink() {
        let face = FaceLandmarks::new(neutral_face()).unwrap();
        let obs = face.observe(Challenge::Blink, 1000, GestureThresholds::default());
        assert_eq!(
            obs.confidence,
            0.0,
            "EAR {} should be above blink threshold",
            face.eye_aspect_ratio()
        );
    }

    #[test]
    fn closed_eyes_register_a_blink() {
        let mut pts = neutral_face();
        // Collapse vertical eye separation → low EAR.
        pts[LEFT_EYE_BOTTOM] = Point3::new(0.62, 0.402, 0.0);
        pts[RIGHT_EYE_BOTTOM] = Point3::new(0.38, 0.402, 0.0);
        let face = FaceLandmarks::new(pts).unwrap();
        let obs = face.observe(Challenge::Blink, 1000, GestureThresholds::default());
        assert!(
            obs.confidence >= 0.5,
            "closed eyes should blink, EAR={}",
            face.eye_aspect_ratio()
        );
    }

    #[test]
    fn turning_head_right_registers_turn_right_not_left() {
        let mut pts = neutral_face();
        // Move nose toward image-right cheek ⇒ positive yaw ⇒ TurnRight.
        pts[NOSE_TIP] = Point3::new(0.70, 0.55, 0.0);
        let face = FaceLandmarks::new(pts).unwrap();
        let right = face.observe(Challenge::TurnRight, 1000, GestureThresholds::default());
        let left = face.observe(Challenge::TurnLeft, 1000, GestureThresholds::default());
        assert!(right.confidence >= 0.5, "yaw={}", face.yaw_ratio());
        assert_eq!(left.confidence, 0.0);
    }

    #[test]
    fn wide_mouth_registers_a_smile() {
        let mut pts = neutral_face();
        // Widen the mouth corners, keep it not-tall → high width/height ratio.
        pts[MOUTH_LEFT] = Point3::new(0.72, 0.70, 0.0);
        pts[MOUTH_RIGHT] = Point3::new(0.28, 0.70, 0.0);
        pts[MOUTH_TOP] = Point3::new(0.50, 0.695, 0.0);
        pts[MOUTH_BOTTOM] = Point3::new(0.50, 0.705, 0.0);
        let face = FaceLandmarks::new(pts).unwrap();
        let obs = face.observe(Challenge::Smile, 1000, GestureThresholds::default());
        assert!(obs.confidence >= 0.5, "MAR={}", face.mouth_aspect_ratio());
    }

    #[test]
    fn derived_observations_drive_the_liveness_fsm_to_pass() {
        // End-to-end: landmark frames → observations → FSM passes.
        let mut session = LivenessSession::new(LivenessConfig {
            challenges: vec![Challenge::Blink, Challenge::TurnLeft],
            ..Default::default()
        })
        .unwrap();
        session.start(500);
        let th = GestureThresholds::default();

        // Blink frame.
        let mut blink = neutral_face();
        blink[LEFT_EYE_BOTTOM] = Point3::new(0.62, 0.402, 0.0);
        blink[RIGHT_EYE_BOTTOM] = Point3::new(0.38, 0.402, 0.0);
        let obs = FaceLandmarks::new(blink)
            .unwrap()
            .observe(Challenge::Blink, 1000, th);
        assert_eq!(session.submit(obs), SubmitOutcome::Advanced);

        // Turn-left frame (nose toward image-left).
        let mut turn = neutral_face();
        turn[NOSE_TIP] = Point3::new(0.30, 0.55, 0.0);
        let obs = FaceLandmarks::new(turn)
            .unwrap()
            .observe(Challenge::TurnLeft, 2000, th);
        assert_eq!(session.submit(obs), SubmitOutcome::Passed);
    }
}
