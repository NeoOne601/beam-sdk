// core/src/quality.rs
// Frame quality gates. These run in strict order on the Y-plane only,
// and SHORT-CIRCUIT: a frame rejected at gate N never reaches gate N+1.
// This is the primary performance lever on budget devices (Helio G85).
// NO GPU involvement here — these run on CPU and must complete < 4ms on
// a Cortex-A55 core to preserve 25fps pipeline throughput.

use crate::frame::RawFrame;

/// Ordered quality gates. Run in this exact sequence — cheapest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Gate {
    /// Frame arrived. Always passes. Entry point.
    Received       = 0,
    /// Reject frames with motion blur above threshold.
    /// Uses Laplacian variance on a 64x64 centre crop of the Y plane.
    BlurCheck      = 1,
    /// Reject underexposed or overexposed frames.
    /// Mean and 95th-percentile luminance check on Y plane histogram.
    ExposureCheck  = 2,
    /// Reject frames where inter-frame motion is too high (camera shake).
    /// Compares 8x8 block SAD against previous accepted frame.
    MotionCheck    = 3,
    /// Document boundary must be detectable (fast edge density heuristic,
    /// NOT the ML model — model is only called after all gates pass).
    BoundaryCheck  = 4,
    /// All gates passed. Frame is forwarded to ML inference.
    Accepted       = 5,
}

#[derive(Debug, Clone)]
pub struct QualityReport {
    pub gate_reached:    Gate,
    /// Laplacian variance (higher = sharper). Threshold: 80.0
    pub blur_score:      f32,
    /// Mean luminance 0–255. Accept range: 40–220.
    pub mean_luma:       f32,
    /// 95th-percentile luminance. Reject if > 245 (overexposed).
    pub p95_luma:        f32,
    /// Inter-frame SAD normalised 0–1. Reject if > 0.12.
    pub motion_score:    f32,
    /// Edge density 0–1. Reject if < 0.08 (no document visible).
    pub edge_density:    f32,
}

pub struct QualityGate {
    pub blur_threshold:    f32,   // default 80.0
    pub min_luma:          f32,   // default 40.0
    pub max_luma:          f32,   // default 220.0
    pub p95_luma_max:      f32,   // default 245.0
    pub motion_threshold:  f32,   // default 0.12
    pub edge_min:          f32,   // default 0.08
    prev_y_crop:           [u8; 64 * 64],
}

impl Default for QualityGate {
    fn default() -> Self {
        Self {
            blur_threshold:   80.0,
            min_luma:         40.0,
            max_luma:         220.0,
            p95_luma_max:     245.0,
            motion_threshold: 0.12,
            edge_min:         0.08,
            prev_y_crop:      [128u8; 64 * 64],
        }
    }
}

impl QualityGate {
    /// Evaluate all gates. Returns early on first rejection.
    /// Only accesses `frame.y_plane` — UV plane never touched here.
    ///
    /// # Safety
    /// Caller guarantees y_plane is valid for width*height bytes.
    pub unsafe fn evaluate(&mut self, frame: &RawFrame) -> QualityReport {
        let mut report = QualityReport {
            gate_reached: Gate::Received,
            blur_score:   0.0,
            mean_luma:    0.0,
            p95_luma:     0.0,
            motion_score: 0.0,
            edge_density: 0.0,
        };

        // --- Gate 1: Blur (Laplacian variance on 64x64 centre crop) ---
        let crop = self.extract_centre_crop_64(frame);
        report.blur_score = laplacian_variance(&crop);
        if report.blur_score < self.blur_threshold {
            report.gate_reached = Gate::BlurCheck;
            return report; // SHORT-CIRCUIT: exposure/motion/edge scores stay 0.0
        }

        // --- Gate 2: Exposure (histogram on full Y plane) ---
        let (mean, p95) = luma_stats(frame);
        report.mean_luma = mean;
        report.p95_luma  = p95;
        if mean < self.min_luma || mean > self.max_luma || p95 > self.p95_luma_max {
            report.gate_reached = Gate::ExposureCheck;
            return report; // SHORT-CIRCUIT: motion/edge scores stay 0.0
        }

        // --- Gate 3: Motion (SAD vs previous accepted crop) ---
        report.motion_score = block_sad_normalised(&crop, &self.prev_y_crop);
        if report.motion_score > self.motion_threshold {
            report.gate_reached = Gate::MotionCheck;
            return report; // SHORT-CIRCUIT: edge score stays 0.0
        }
        self.prev_y_crop = crop;

        // --- Gate 4: Boundary (Sobel edge density) ---
        report.edge_density = sobel_edge_density(frame);
        if report.edge_density < self.edge_min {
            report.gate_reached = Gate::BoundaryCheck;
            return report;
        }

        report.gate_reached = Gate::Accepted;
        report
    }

    unsafe fn extract_centre_crop_64(&self, frame: &RawFrame) -> [u8; 64 * 64] {
        let mut crop = [0u8; 64 * 64];
        let x0 = (frame.width  / 2).saturating_sub(32) as usize;
        let y0 = (frame.height / 2).saturating_sub(32) as usize;
        for row in 0..64usize {
            let src_offset = (y0 + row) * frame.y_stride as usize + x0;
            let dst_offset = row * 64;
            core::ptr::copy_nonoverlapping(
                frame.y_plane.add(src_offset),
                crop.as_mut_ptr().add(dst_offset),
                64,
            );
        }
        crop
    }
}

/// Laplacian variance — sharpness metric.
/// Kernel: [0,1,0 / 1,-4,1 / 0,1,0]
fn laplacian_variance(crop: &[u8; 64 * 64]) -> f32 {
    let mut sum: f64 = 0.0;
    let mut sum_sq: f64 = 0.0;
    let n = (62 * 62) as f64;
    for y in 1..63usize {
        for x in 1..63usize {
            let idx = y * 64 + x;
            let lap = -(4i16 * crop[idx] as i16)
                + crop[idx - 64] as i16
                + crop[idx + 64] as i16
                + crop[idx - 1]  as i16
                + crop[idx + 1]  as i16;
            let lf = lap as f64;
            sum    += lf;
            sum_sq += lf * lf;
        }
    }
    let variance = (sum_sq - sum * sum / n) / n;
    variance as f32
}

/// Mean and 95th-percentile luminance via histogram (O(n) single pass).
unsafe fn luma_stats(frame: &RawFrame) -> (f32, f32) {
    let mut hist = [0u32; 256];
    let total = (frame.width * frame.height) as usize;
    let y_slice = core::slice::from_raw_parts(frame.y_plane, total);
    for &pixel in y_slice {
        hist[pixel as usize] += 1;
    }
    let mean = y_slice.iter().map(|&p| p as u64).sum::<u64>() as f32 / total as f32;
    let p95_count = (total as f32 * 0.95) as u32;
    let mut cumsum = 0u32;
    let mut p95 = 255f32;
    for (luma, &count) in hist.iter().enumerate() {
        cumsum += count;
        if cumsum >= p95_count {
            p95 = luma as f32;
            break;
        }
    }
    (mean, p95)
}

/// Normalised sum of absolute differences between two 64x64 luma crops.
fn block_sad_normalised(a: &[u8; 64 * 64], b: &[u8; 64 * 64]) -> f32 {
    let sad: u32 = a.iter().zip(b.iter())
        .map(|(&x, &y)| (x as i16 - y as i16).unsigned_abs() as u32)
        .sum();
    sad as f32 / (64.0 * 64.0 * 255.0)
}

/// Sobel edge density on the full Y plane (subsampled 4x for speed).
unsafe fn sobel_edge_density(frame: &RawFrame) -> f32 {
    let w = frame.width as usize;
    let h = frame.height as usize;
    let stride = frame.y_stride as usize;
    let y_plane = core::slice::from_raw_parts(frame.y_plane, stride * h);
    let threshold = 30i16;
    let mut edge_count = 0u32;
    let mut total = 0u32;
    let step = 4; // subsample 4x for budget device compliance
    for y in (1..h - 1).step_by(step) {
        for x in (1..w - 1).step_by(step) {
            let gx = y_plane[(y-1)*stride + (x+1)] as i16
                   - y_plane[(y-1)*stride + (x-1)] as i16
                   + 2 * y_plane[y*stride + (x+1)] as i16
                   - 2 * y_plane[y*stride + (x-1)] as i16
                   + y_plane[(y+1)*stride + (x+1)] as i16
                   - y_plane[(y+1)*stride + (x-1)] as i16;
            let gy = y_plane[(y-1)*stride + (x-1)] as i16
                   + 2 * y_plane[(y-1)*stride + x] as i16
                   + y_plane[(y-1)*stride + (x+1)] as i16
                   - y_plane[(y+1)*stride + (x-1)] as i16
                   - 2 * y_plane[(y+1)*stride + x] as i16
                   - y_plane[(y+1)*stride + (x+1)] as i16;
            let mag = gx.abs().saturating_add(gy.abs());
            if mag > threshold { edge_count += 1; }
            total += 1;
        }
    }
    if total == 0 { return 0.0; }
    edge_count as f32 / total as f32
}
