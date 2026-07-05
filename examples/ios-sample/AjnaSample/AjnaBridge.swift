import Foundation

/// Swift wrapper over the `ajna-core` C FFI (declared in `include/ajna_ffi.h`,
/// exposed to Swift via the bridging header). The Rust core owns quality gates,
/// the session state machine, ML-DSA-65 signing, and canonical-bytes encoding;
/// Swift only marshals frames and reads the signed JSON back.
///
/// The `ajna_*` C symbols are provided by `AjnaCore.xcframework`. This file
/// gives them a safe, idiomatic Swift surface.
enum AjnaBridge {
    /// Returns true when the declarative UiConfig JSON passes `ajna_ui_config_validate`.
    static func validateUiConfig(_ json: String) -> Bool {
        var bytes = Array(json.utf8)
        // AJNA_OK == 0
        return ajna_ui_config_validate(&bytes, bytes.count) == 0
    }

    /// Open a scan session. Returns an opaque handle, or nil on failure.
    static func createSession(minQualityFrames: Int32 = 3, timeoutMs: UInt64 = 30_000) -> OpaquePointer? {
        var config = AjnaSessionConfig()
        config.min_quality_frames = minQualityFrames
        config.timeout_ms = timeoutMs
        config.pqc_sign_result = true
        return ajna_session_create(config)
    }

    /// Fetch the completed, ML-DSA-65-signed ScanResult as JSON.
    static func resultJSON(_ handle: OpaquePointer) -> String? {
        var buffer = [UInt8](repeating: 0, count: 8192)
        let written = ajna_session_get_result_json(handle, &buffer, buffer.count)
        guard written > 0 else { return nil }
        return String(bytes: buffer[0..<Int(written)], encoding: .utf8)
    }

    static func destroy(_ handle: OpaquePointer) {
        _ = ajna_session_destroy(handle)
    }
}
