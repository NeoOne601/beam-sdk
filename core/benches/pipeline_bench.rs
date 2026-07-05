// core/benches/pipeline_bench.rs
// Criterion benchmarks for the Ajna quality gate pipeline.
// Targets documented in AJNA_VERIFY_PRODUCT_PROMPT.md:
//   - blur_gate_1080p: p99 < 4ms (Cortex-A55 scaled from host)
//   - full_gate_pipeline_1080p: p99 < 8ms
//   - pipeline_rejected_at_blur: < 500µs (short-circuit)
//   - session_state_machine: < 100µs per lifecycle

use ajna_core::result::{DocumentField, ScanResult};
use ajna_core::{PixelFormat, QualityGate, RawFrame, ScanSession, SessionConfig, SessionState};
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::time::Duration;

/// Create a checkerboard Y-plane pattern for realistic variance.
/// Alternating 220/30 values prevent blur gate from rejecting (high Laplacian variance).
fn make_checkerboard_frame(w: u32, h: u32) -> (Vec<u8>, Vec<u8>) {
    let y: Vec<u8> = (0..(w * h) as usize)
        .map(|i| {
            if (i / w as usize + i % w as usize).is_multiple_of(2) {
                220
            } else {
                30
            }
        })
        .collect();
    let uv = vec![128u8; (w * h / 2) as usize];
    (y, uv)
}

/// Create a constant-value Y-plane (guaranteed blur rejection — near-zero Laplacian variance).
fn make_constant_frame(w: u32, h: u32, value: u8) -> (Vec<u8>, Vec<u8>) {
    let y = vec![value; (w * h) as usize];
    let uv = vec![128u8; (w * h / 2) as usize];
    (y, uv)
}

pub fn bench_blur_gate_1080p(c: &mut Criterion) {
    let (y, uv) = make_checkerboard_frame(1920, 1080);
    let frame = RawFrame {
        y_plane: y.as_ptr(),
        uv_plane: uv.as_ptr(),
        width: 1920,
        height: 1080,
        y_stride: 1920,
        uv_stride: 1920,
        format: PixelFormat::Nv12,
        timestamp_us: 0,
    };
    let mut gate = QualityGate::default();
    let mut group = c.benchmark_group("quality_gates");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(5));
    group.bench_function("blur_gate_1080p", |b| {
        b.iter(|| unsafe { gate.evaluate(&frame) })
    });
    group.finish();
}

pub fn bench_full_gate_pipeline_1080p(c: &mut Criterion) {
    let (y, uv) = make_checkerboard_frame(1920, 1080);
    let frame = RawFrame {
        y_plane: y.as_ptr(),
        uv_plane: uv.as_ptr(),
        width: 1920,
        height: 1080,
        y_stride: 1920,
        uv_stride: 1920,
        format: PixelFormat::Nv12,
        timestamp_us: 0,
    };
    let mut gate = QualityGate::default();
    // Pre-warm the gate with one frame so motion gate has a reference
    unsafe {
        gate.evaluate(&frame);
    }

    let mut group = c.benchmark_group("quality_gates");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(5));
    group.bench_function("full_gate_pipeline_1080p", |b| {
        b.iter(|| unsafe { gate.evaluate(&frame) })
    });
    group.finish();
}

pub fn bench_pipeline_rejected_at_blur(c: &mut Criterion) {
    // Constant value frame — guaranteed blur rejection (Laplacian variance ≈ 0)
    let (y, uv) = make_constant_frame(1920, 1080, 128);
    let frame = RawFrame {
        y_plane: y.as_ptr(),
        uv_plane: uv.as_ptr(),
        width: 1920,
        height: 1080,
        y_stride: 1920,
        uv_stride: 1920,
        format: PixelFormat::Nv12,
        timestamp_us: 0,
    };
    let mut gate = QualityGate::default();
    let mut group = c.benchmark_group("quality_gates");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(3));
    group.bench_function("rejected_at_blur_1080p", |b| {
        b.iter(|| unsafe { gate.evaluate(&frame) })
    });
    group.finish();
}

pub fn bench_session_state_machine(c: &mut Criterion) {
    let mut group = c.benchmark_group("session");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(3));
    group.bench_function("full_session_lifecycle", |b| {
        b.iter(|| {
            let config = SessionConfig::default();
            let mut session = ScanSession::new(config);
            session.start(1000);
            session.record_quality_frame();
            session.record_quality_frame();
            session.record_quality_frame(); // triggers Inferring
            assert_eq!(session.state, SessionState::Inferring);
            let result = ScanResult {
                fields: vec![DocumentField {
                    key: "surname".into(),
                    value: "SMITH".into(),
                    confidence: 0.99,
                }],
                raw_mrz: None,
                document_type: "passport".into(),
                issuing_country: "USA".into(),
                confidence: 0.98,
                pqc_signature: Vec::new(),
                pqc_public_key: Vec::new(),
                nonce: None,
                session_id: None,
                timestamp_iso: None,
                algo: String::new(),
                ajna_version: String::from("2.0"),
                public_key: String::new(),
                jws_token: None,
            };
            session.complete(result);
            assert_eq!(session.state, SessionState::Complete);
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_blur_gate_1080p,
    bench_full_gate_pipeline_1080p,
    bench_pipeline_rejected_at_blur,
    bench_session_state_machine
);
criterion_main!(benches);
