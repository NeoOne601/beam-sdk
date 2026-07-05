# Ajna Verify SDK — FFI Fuzzing Guide

## Purpose

This guide documents how to fuzz the Ajna Verify FFI boundary — the C-callable functions in `core/src/ffi.rs` — using libFuzzer with AddressSanitizer (ASAN). The FFI boundary is the primary attack surface: it receives untrusted data from the C++ ML bridges.

## Prerequisites

- Rust nightly (for `-Zsanitizer=address`)
- `cargo-fuzz` (`cargo install cargo-fuzz`)
- Clang 14+ with libFuzzer support

## Setup

```bash
cd core
cargo fuzz init
```

## Starter Fuzz Target: `ajna_session_push_result`

This function receives raw CField pointers from C++ code. A malicious or buggy bridge could send:
- NULL key/value pointers
- key_len/value_len exceeding actual buffer size
- Non-UTF-8 byte sequences
- Extremely large field counts

### Create the target

```bash
mkdir -p fuzz/fuzz_targets
```

```rust
// fuzz/fuzz_targets/fuzz_push_result.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use ajna_core::ffi::{CField, ajna_session_create, ajna_session_start, ajna_session_push_result, ajna_session_destroy};

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 { return; }

    // Create a session
    let session = unsafe { ajna_session_create() };
    if session.is_null() { return; }
    unsafe { ajna_session_start(session, 1000); }

    // Parse fuzz data into field-like structures
    let num_fields = (data[0] as usize).min(20);
    let mut fields = Vec::new();
    let mut offset = 1;

    for _ in 0..num_fields {
        if offset + 4 >= data.len() { break; }
        let key_len = (data[offset] as usize).min(64).min(data.len() - offset - 3);
        let val_len = (data[offset + 1] as usize).min(256).min(data.len() - offset - 2 - key_len);
        let key_start = offset + 2;
        let val_start = key_start + key_len;

        if val_start + val_len > data.len() { break; }

        fields.push(CField {
            key: data[key_start..].as_ptr(),
            key_len,
            value: data[val_start..].as_ptr(),
            value_len: val_len,
            confidence: 0.5,
        });
        offset = val_start + val_len;
    }

    // Call the FFI function with fuzzed data
    let doc_type = b"passport";
    let country = b"USA";
    unsafe {
        ajna_session_push_result(
            session,
            fields.as_ptr(),
            fields.len(),
            doc_type.as_ptr(),
            doc_type.len(),
            country.as_ptr(),
            country.len(),
            0.5,
            false,
        );
        ajna_session_destroy(session);
    }
});
```

## Running the Fuzzer

```bash
# With ASAN enabled
cd core
cargo +nightly fuzz run fuzz_push_result -- \
    -max_len=4096 \
    -timeout=10 \
    -jobs=4

# Run for a fixed duration (e.g., 1 hour)
cargo +nightly fuzz run fuzz_push_result -- \
    -max_total_time=3600
```

## Additional Fuzz Targets (Recommended)

| Target | FFI Function | Risk |
|--------|-------------|------|
| `fuzz_gate_evaluate` | `ajna_gate_evaluate` | Malformed RawFrame with invalid pointer/stride |
| `fuzz_field_parser` | `FieldParser::parse` | Invalid UTF-8, extreme lengths, bad date formats |
| `fuzz_canonical_bytes` | `ScanResult::canonical_bytes` | Empty fields, maximum field count |

## Interpreting Results

- **ASAN violations**: Use-after-free, buffer overflow, stack overflow → fix in FFI boundary checks
- **Timeouts**: Algorithmic complexity issues (unlikely for Ajna's O(n) pipelines)
- **Panics**: Unwinding across FFI boundary is UB — ensure all FFI functions catch panics

## CI Integration

Add to `.github/workflows/ci.yml`:

```yaml
fuzz-test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@nightly
    - run: cargo install cargo-fuzz
    - run: cd core && cargo +nightly fuzz run fuzz_push_result -- -max_total_time=300
```
