// tests/quality_gate_tests.rs
// Quality gate test suite.
// Tests must be compiled against the core library; they operate on public types.
//
// CRITICAL INVARIANT: when a frame fails Gate::BlurCheck, the returned QualityReport
// MUST have exposure_score == 0.0, motion_score == 0.0, and edge_density == 0.0.
// This short-circuit is the performance guarantee for budget devices.

#[cfg(test)]
mod tests {
    use beam_core::frame::{PixelFormat, RawFrame};
    use beam_core::quality::{Gate, QualityGate};

    // ─── Helper: create a synthetic RawFrame from a Vec<u8> Y plane ──────────

    fn make_frame(y_data: &[u8], width: u32, height: u32) -> (Vec<u8>, Vec<u8>, RawFrame) {
        assert_eq!(y_data.len(), (width * height) as usize);
        let y = y_data.to_vec();
        let uv = vec![128u8; (width * height / 2) as usize]; // neutral UV
        let frame = RawFrame {
            y_plane: y.as_ptr(),
            uv_plane: uv.as_ptr(),
            width,
            height,
            y_stride: width,
            uv_stride: width,
            format: PixelFormat::Nv12,
            timestamp_us: 0,
        };
        (y, uv, frame) // return vecs so they stay alive
    }

    // ─── Sharp frame (high Laplacian variance) ────────────────────────────────

    #[test]
    fn sharp_frame_passes_blur_gate() {
        // Checkerboard pattern → very high Laplacian variance
        let width = 200u32;
        let height = 200u32;
        let y_data: Vec<u8> = (0..(width * height) as usize)
            .map(|i| {
                let row = i / width as usize;
                let col = i % width as usize;
                if (row + col) % 2 == 0 {
                    240
                } else {
                    10
                }
            })
            .collect();

        let (y, uv, frame) = make_frame(&y_data, width, height);
        let mut gate = QualityGate::default();
        let report = unsafe { gate.evaluate(&frame) };
        drop((y, uv));

        println!("blur_score = {}", report.blur_score);
        assert!(
            report.blur_score > 80.0,
            "sharp checkerboard must have blur_score > 80.0, got {}",
            report.blur_score
        );
    }

    // ─── Blurred frame (constant Y plane) ────────────────────────────────────

    #[test]
    fn blurred_frame_fails_blur_gate() {
        let width = 200u32;
        let height = 200u32;
        // Constant Y = 128 → Laplacian = 0 everywhere → variance ≈ 0
        let y_data = vec![128u8; (width * height) as usize];

        let (y, uv, frame) = make_frame(&y_data, width, height);
        let mut gate = QualityGate::default();
        let report = unsafe { gate.evaluate(&frame) };
        drop((y, uv));

        println!("blur_score = {}", report.blur_score);
        assert!(
            report.blur_score < 5.0,
            "constant Y frame must have blur_score < 5.0, got {}",
            report.blur_score
        );
        assert_eq!(report.gate_reached, Gate::BlurCheck);
    }

    // ─── Dark frame fails exposure gate ──────────────────────────────────────

    #[test]
    fn dark_frame_fails_exposure_gate() {
        let width = 200u32;
        let height = 200u32;
        // Mean luma ≈ 20 (dark) → fails min_luma = 40
        // But must pass blur first → use checkerboard with dark values
        let y_data: Vec<u8> = (0..(width * height) as usize)
            .map(|i| {
                let row = i / width as usize;
                let col = i % width as usize;
                if (row + col) % 2 == 0 {
                    30
                } else {
                    10
                } // mean ≈ 20
            })
            .collect();

        let (y, uv, frame) = make_frame(&y_data, width, height);
        let mut gate = QualityGate::default();
        gate.blur_threshold = 1.0; // relax blur so we reach exposure check
        let report = unsafe { gate.evaluate(&frame) };
        drop((y, uv));

        println!("mean_luma = {}", report.mean_luma);
        assert_eq!(
            report.gate_reached,
            Gate::ExposureCheck,
            "dark frame (mean≈20) must fail exposure gate"
        );
        assert!(report.mean_luma < 40.0);
    }

    // ─── Overexposed frame fails exposure gate ────────────────────────────────

    #[test]
    fn overexposed_frame_fails_exposure_gate() {
        let width = 200u32;
        let height = 200u32;
        // P95 luma > 248 → fails p95_luma_max = 245
        let y_data: Vec<u8> = (0..(width * height) as usize)
            .map(|i| {
                if i < (width * height * 96 / 100) as usize {
                    250
                } else {
                    100
                }
            })
            .collect();

        let (y, uv, frame) = make_frame(&y_data, width, height);
        let mut gate = QualityGate::default();
        gate.blur_threshold = -1.0; // relax blur
        let report = unsafe { gate.evaluate(&frame) };
        drop((y, uv));

        println!("p95_luma = {}", report.p95_luma);
        assert_eq!(
            report.gate_reached,
            Gate::ExposureCheck,
            "overexposed frame must fail exposure gate"
        );
    }

    // ─── Identical consecutive crops → low motion score ──────────────────────

    #[test]
    fn identical_frames_pass_motion_gate() {
        let width = 200u32;
        let height = 200u32;
        let y_data: Vec<u8> = (0..(width * height) as usize)
            .map(|i| {
                let row = i / width as usize;
                let col = i % width as usize;
                if (row + col) % 2 == 0 {
                    200
                } else {
                    50
                }
            })
            .collect();

        let mut gate = QualityGate::default();
        gate.blur_threshold = -1.0;
        gate.min_luma = 1.0;
        gate.motion_threshold = 1.0;

        // First frame: initialise prev_y_crop
        let (y, uv, frame) = make_frame(&y_data, width, height);
        let _ = unsafe { gate.evaluate(&frame) };
        drop((y, uv));

        // Second identical frame: SAD should be ~0
        let (y, uv, frame) = make_frame(&y_data, width, height);
        let report = unsafe { gate.evaluate(&frame) };
        drop((y, uv));

        println!("motion_score = {}", report.motion_score);
        assert!(
            report.motion_score < 0.01,
            "identical frames must have motion_score < 0.01, got {}",
            report.motion_score
        );
    }

    // ─── Random crop vs previous → high motion score ─────────────────────────

    #[test]
    fn random_frames_fail_motion_gate() {
        let width = 200u32;
        let height = 200u32;

        // Frame 1: all 50
        let y1 = vec![50u8; (width * height) as usize];
        // Frame 2: all 200
        let y2 = vec![200u8; (width * height) as usize];

        let mut gate = QualityGate::default();
        gate.blur_threshold = -1.0;
        gate.min_luma = 1.0;

        let (y_buf, uv_buf, frame) = make_frame(&y1, width, height);
        let _ = unsafe { gate.evaluate(&frame) };
        drop((y_buf, uv_buf));

        let (y_buf, uv_buf, frame) = make_frame(&y2, width, height);
        let report = unsafe { gate.evaluate(&frame) };
        drop((y_buf, uv_buf));

        println!("motion_score = {}", report.motion_score);
        assert!(
            report.motion_score > 0.12,
            "maximally different frames must fail motion gate (>0.12), got {}",
            report.motion_score
        );
        assert_eq!(report.gate_reached, Gate::MotionCheck);
    }

    // ─── Flat grey frame fails boundary gate ─────────────────────────────────

    #[test]
    fn flat_frame_fails_boundary_gate() {
        let width = 200u32;
        let height = 200u32;
        // Constant 128 → Sobel magnitude everywhere = 0 → edge_density = 0
        let y_data = vec![128u8; (width * height) as usize];

        let mut gate = QualityGate::default();
        gate.blur_threshold = -1.0;
        gate.min_luma = 1.0;
        gate.motion_threshold = 1.0;

        let (y, uv, frame) = make_frame(&y_data, width, height);
        let report = unsafe { gate.evaluate(&frame) };
        drop((y, uv));

        println!("edge_density = {}", report.edge_density);
        assert!(
            report.edge_density < 0.02,
            "flat grey frame must have edge_density < 0.02, got {}",
            report.edge_density
        );
        assert_eq!(report.gate_reached, Gate::BoundaryCheck);
    }

    // ─── White rect on black passes boundary gate ─────────────────────────────

    #[test]
    fn rect_frame_passes_boundary_gate() {
        let width = 200u32;
        let height = 200u32;
        // Document-like: black background with a central white rectangle
        let mut y_data = vec![0u8; (width * height) as usize];
        for row in 60..140usize {
            for col in 50..150usize {
                y_data[row * width as usize + col] = 240;
            }
        }

        let mut gate = QualityGate::default();
        gate.blur_threshold = -1.0;
        gate.min_luma = 1.0;
        gate.motion_threshold = 1.0;

        let (y, uv, frame) = make_frame(&y_data, width, height);
        let report = unsafe { gate.evaluate(&frame) };
        drop((y, uv));

        println!("edge_density = {}", report.edge_density);
        assert!(
            report.edge_density > 0.01,
            "white-rect-on-black must have edge_density > 0.01, got {}",
            report.edge_density
        );
    }

    // ─── CRITICAL INVARIANT: blur rejection zeros all downstream scores ────────

    #[test]
    fn blur_rejection_zeros_downstream_scores() {
        let width = 200u32;
        let height = 200u32;
        // Constant Y = 128 → guaranteed blur rejection
        let y_data = vec![128u8; (width * height) as usize];

        let (y, uv, frame) = make_frame(&y_data, width, height);
        let mut gate = QualityGate::default(); // default blur_threshold = 80.0
        let report = unsafe { gate.evaluate(&frame) };
        drop((y, uv));

        assert_eq!(
            report.gate_reached,
            Gate::BlurCheck,
            "constant frame must fail at BlurCheck"
        );
        // CRITICAL: short-circuit means these must ALL be exactly 0.0
        assert_eq!(
            report.mean_luma, 0.0,
            "mean_luma must be 0.0 when rejected at BlurCheck (not yet computed)"
        );
        assert_eq!(
            report.p95_luma, 0.0,
            "p95_luma must be 0.0 when rejected at BlurCheck"
        );
        assert_eq!(
            report.motion_score, 0.0,
            "motion_score must be 0.0 when rejected at BlurCheck"
        );
        assert_eq!(
            report.edge_density, 0.0,
            "edge_density must be 0.0 when rejected at BlurCheck"
        );
    }
}
