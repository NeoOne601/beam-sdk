// core/benches/crypto_bench.rs
// Criterion benchmarks for PQC cryptographic operations.
// Targets documented in BEAM_VERIFY_PRODUCT_PROMPT.md:
//   - ml_dsa_keygen: < 5ms per keygen on host
//   - ml_dsa_sign: < 5ms per sign on host
//   - ml_dsa_verify: < 3ms per verify on host
//   - canonical_bytes: < 100µs with 9 fields
//   - ml_kem_encapsulate: < 3ms per encapsulation

use beam_core::result::{DocumentField, ScanResult};
use beam_core::{MlDsaLevel, MlKemSession, PqcSigner};
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::time::Duration;

pub fn bench_ml_dsa_keygen(c: &mut Criterion) {
    let mut group = c.benchmark_group("pqc_crypto");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(5));
    group.bench_function("ml_dsa_keygen_level3", |b| {
        b.iter(|| PqcSigner::generate(MlDsaLevel::Level3).expect("keygen failed"))
    });
    group.finish();
}

pub fn bench_ml_dsa_sign(c: &mut Criterion) {
    let signer = PqcSigner::generate(MlDsaLevel::Level3).expect("keygen failed");
    let message = vec![0xABu8; 256]; // 256-byte message

    let mut group = c.benchmark_group("pqc_crypto");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(5));
    group.bench_function("ml_dsa_sign_256b", |b| {
        b.iter(|| signer.sign(&message).expect("sign failed"))
    });
    group.finish();
}

pub fn bench_ml_dsa_verify(c: &mut Criterion) {
    let signer = PqcSigner::generate(MlDsaLevel::Level3).expect("keygen failed");
    let message = vec![0xABu8; 256];
    let signature = signer.sign(&message).expect("sign failed");
    let public_key = signer.public_key_bytes().to_vec();

    let mut group = c.benchmark_group("pqc_crypto");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(5));
    group.bench_function("ml_dsa_verify", |b| {
        b.iter(|| {
            PqcSigner::verify(MlDsaLevel::Level3, &public_key, &message, &signature)
                .expect("verify failed")
        })
    });
    group.finish();
}

pub fn bench_canonical_bytes(c: &mut Criterion) {
    let result = ScanResult {
        fields: vec![
            DocumentField {
                key: "surname".into(),
                value: "ERIKSSON".into(),
                confidence: 0.97,
            },
            DocumentField {
                key: "given_names".into(),
                value: "ANNA MARIA".into(),
                confidence: 0.96,
            },
            DocumentField {
                key: "date_of_birth".into(),
                value: "1974-08-12".into(),
                confidence: 0.99,
            },
            DocumentField {
                key: "document_number".into(),
                value: "L898902C3".into(),
                confidence: 0.98,
            },
            DocumentField {
                key: "expiry_date".into(),
                value: "2012-04-15".into(),
                confidence: 0.95,
            },
            DocumentField {
                key: "mrz_line1".into(),
                value: "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<".into(),
                confidence: 0.99,
            },
            DocumentField {
                key: "mrz_line2".into(),
                value: "L898902C36UTO7408122F1204159ZE184226B<<<<<10".into(),
                confidence: 0.99,
            },
            DocumentField {
                key: "surname".into(),
                value: "SMITH".into(),
                confidence: 0.99,
            },
            DocumentField {
                key: "given_names".into(),
                value: "JOHN DOE".into(),
                confidence: 0.98,
            },
        ],
        raw_mrz: None,
        document_type: "passport".into(),
        issuing_country: "USA".into(),
        confidence: 0.99,
        pqc_signature: Vec::new(),
        pqc_public_key: Vec::new(),
        nonce: None,
        session_id: None,
        timestamp_iso: None,
        algo: String::new(),
        beam_version: String::from("2.0"),
        public_key: String::new(),
        jws_token: None,
    };

    let mut group = c.benchmark_group("pqc_crypto");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(3));
    group.bench_function("canonical_bytes_9_fields", |b| {
        b.iter(|| result.canonical_bytes())
    });
    group.finish();
}

pub fn bench_ml_kem_encapsulate(c: &mut Criterion) {
    // Generate a KEM keypair for benchmarking
    use pqcrypto_kyber::kyber1024;
    use pqcrypto_traits::kem::PublicKey as _;
    let (pk, _sk) = kyber1024::keypair();
    let pk_bytes = pk.as_bytes().to_vec();

    let mut group = c.benchmark_group("pqc_crypto");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(5));
    group.bench_function("ml_kem_encapsulate", |b| {
        b.iter(|| MlKemSession::encapsulate(&pk_bytes).expect("encapsulate failed"))
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_ml_dsa_keygen,
    bench_ml_dsa_sign,
    bench_ml_dsa_verify,
    bench_canonical_bytes,
    bench_ml_kem_encapsulate
);
criterion_main!(benches);
