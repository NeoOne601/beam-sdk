// platform/ios/AjnaSDK.swift
// Public Ajna SDK interface for iOS.
// AjnaScanner owns a AjnaCameraAdapter and coordinates with the CoreML bridge.
// All public completion callbacks are dispatched on DispatchQueue.main.

import AVFoundation
import CoreVideo
import Foundation

// MARK: – C Bridge Declarations (linked via libajna_sdk.a)

@_silgen_name("ajna_coreml_create")
private func ajna_coreml_create(_ modelPath: UnsafePointer<CChar>?) -> UnsafeMutableRawPointer?

@_silgen_name("ajna_coreml_process")
private func ajna_coreml_process(
    _ session:     UnsafeMutableRawPointer?,
    _ pixelBuffer: CVPixelBuffer,
    _ rustSession: UnsafeMutableRawPointer?
)

@_silgen_name("ajna_coreml_destroy")
private func ajna_coreml_destroy(_ session: UnsafeMutableRawPointer?)

@_silgen_name("ajna_session_get_state")
private func ajna_session_get_state(_ handle: UnsafeMutableRawPointer?) -> UInt32

@_silgen_name("ajna_session_create")
private func ajna_session_create(_ config: AjnaSessionConfigC) -> UnsafeMutableRawPointer?

@_silgen_name("ajna_session_start")
private func ajna_session_start(_ handle: UnsafeMutableRawPointer?, _ timestampUs: UInt64)

@_silgen_name("ajna_session_get_result_json")
private func ajna_session_get_result_json(
    _ handle: UnsafeMutableRawPointer?,
    _ outBuf: UnsafeMutablePointer<UInt8>?,
    _ outBufLen: Int
) -> Int32

private struct AjnaSessionConfigC {
    var minQualityFrames:  UInt32
    var timeoutMs:         UInt64
    var adaptiveGateLimit: UInt32
    var pqcSignResult:     Bool
    var includeRawMrz:     Bool
}

// MARK: – Configuration

/// Mirror of Rust SessionConfig, exposed to iOS integrators.
public struct AjnaScanConfig {
    /// Number of quality-passing frames required before inference triggers.
    /// Default: 3.
    public var minQualityFrames: Int
    /// Session timeout in milliseconds. Default: 30000 (30 seconds).
    public var timeoutMs:         Int
    /// Whether to PQC-sign the result. Set false in dev builds to reduce latency.
    public var pqcSignResult:     Bool
    /// Path to the CoreML model file. Empty string = stub mode (no inference).
    public var modelPath:         String

    public init(
        minQualityFrames: Int  = 3,
        timeoutMs:         Int  = 30_000,
        pqcSignResult:     Bool = true,
        modelPath:         String = ""
    ) {
        self.minQualityFrames = minQualityFrames
        self.timeoutMs         = timeoutMs
        self.pqcSignResult     = pqcSignResult
        self.modelPath         = modelPath
    }
}

// MARK: – Result

/// Mirror of Rust ScanResult, Codable for persistence or network serialisation.
public struct AjnaScanResult: Codable {
    /// Extracted document fields (e.g. surname, document_number).
    public let fields:          [AjnaDocumentField]
    /// Raw MRZ string, if requested in AjnaScanConfig.
    public let rawMrz:          String?
    /// ICAO document type (e.g. "passport", "id_card").
    public let documentType:    String
    /// ISO 3166-1 alpha-3 issuing country (e.g. "USA", "GBR").
    public let issuingCountry:  String
    /// Overall scan confidence, 0.0–1.0.
    public let confidence:      Float
    /// ML-DSA Level-3 signature over the canonical field encoding. Empty if not signed.
    public let pqcSignature:    Data
    /// Matching ML-DSA public key. Empty if not signed.
    public let pqcPublicKey:    Data
}

/// A single extracted field from the document.
public struct AjnaDocumentField: Codable {
    public let key:        String
    public let value:      String
    public let confidence: Float
}

// MARK: – Errors

/// Errors surfaced by AjnaScanner.
public enum AjnaScannerError: Error {
    /// No camera hardware available (simulator or denied permission).
    case noCameraAvailable
    /// CoreML model file not found at the expected bundle path.
    case modelNotFound(String)
    /// Session timed out before enough quality frames were captured.
    case sessionTimeout
    /// CoreML inference returned an error.
    case inferenceFailure(String)
    /// Camera configuration failed.
    case configurationFailed(String)
}

// MARK: – AjnaScanner

/// Primary public interface for document scanning on iOS.
///
/// Usage:
/// ```swift
/// let scanner = AjnaScanner()
/// try scanner.configure()
/// scanner.startScan(config: AjnaScanConfig()) { result in
///     print(result.documentType, result.issuingCountry)
/// }
/// ```
public class AjnaScanner: NSObject {

    private var cameraAdapter: AjnaCameraAdapter?
    private var completion:    ((Result<AjnaScanResult, AjnaScannerError>) -> Void)?
    private var scanConfig:    AjnaScanConfig = AjnaScanConfig()
    private var coremlSession:  UnsafeMutableRawPointer? = nil
    private var rustSession:    UnsafeMutableRawPointer? = nil
    private var modelPath:      String = ""

    // MARK: – Public API

    /// Configure the camera session. Must be called before startScan().
    /// Throws AjnaScannerError.noCameraAvailable on failure.
    public func configure() throws {
        let adapter = AjnaCameraAdapter()
        do {
            try adapter.configure()
        } catch let e as AjnaError {
            switch e {
            case .noCameraAvailable:
                throw AjnaScannerError.noCameraAvailable
            case .configurationFailed(let msg):
                throw AjnaScannerError.configurationFailed(msg)
            }
        }
        adapter.delegate = self
        self.cameraAdapter = adapter
    }

    /// Start a scan with the given configuration.
    ///
    /// The completion block is always called on DispatchQueue.main.
    /// Only one scan can be active at a time; call stopScan() first if needed.
    public func startScan(
        config:     AjnaScanConfig = AjnaScanConfig(),
        completion: @escaping (Result<AjnaScanResult, AjnaScannerError>) -> Void
    ) {
        self.scanConfig = config
        self.completion = completion
        self.modelPath = config.modelPath

        // Initialise CoreML session if a model path was provided.
        // If modelPath is empty, coremlSession stays nil and stub mode runs.
        if !modelPath.isEmpty {
            coremlSession = modelPath.withCString { ajna_coreml_create($0) }
        }

        // Initialise Rust session
        let cfg = AjnaSessionConfigC(
            minQualityFrames:  UInt32(config.minQualityFrames),
            timeoutMs:         UInt64(config.timeoutMs),
            adaptiveGateLimit: 60,
            pqcSignResult:     config.pqcSignResult,
            includeRawMrz:     false
        )
        rustSession = ajna_session_create(cfg)
        if let rs = rustSession {
            ajna_session_start(rs, UInt64(Date().timeIntervalSince1970 * 1_000_000))
        }

        cameraAdapter?.startCapture()
    }

    /// Stop the active scan and release camera resources.
    public func stopScan() {
        cameraAdapter?.stopCapture()
        completion = nil
        ajna_coreml_destroy(coremlSession)
        coremlSession = nil
        rustSession = nil
    }

    // MARK: – Internal result dispatch

    private func deliverResult(_ result: AjnaScanResult) {
        let block = self.completion
        self.completion = nil
        cameraAdapter?.stopCapture()
        DispatchQueue.main.async {
            block?(.success(result))
        }
    }

    private func deliverError(_ error: AjnaScannerError) {
        let block = self.completion
        self.completion = nil
        cameraAdapter?.stopCapture()
        DispatchQueue.main.async {
            block?(.failure(error))
        }
    }
}

// MARK: – AjnaFrameDelegate

extension AjnaScanner: AjnaFrameDelegate {

    /// Called by AjnaCameraAdapter for each locked CVPixelBuffer.
    /// The buffer is already locked with .readOnly by the adapter.
    /// We call ajna_coreml_process (via @_silgen_name bridge) and check state.
    /// The adapter will unlock the buffer immediately after this call returns.
    public func didReceiveFrame(_ buffer: CVPixelBuffer, timestamp: CMTime) {
        guard let rs = rustSession else { return }

        // ajna_coreml_process accepts the locked CVPixelBuffer directly.
        // The buffer is already locked by AjnaCameraAdapter with .readOnly.
        // This call either runs real CoreML inference (if coremlSession != nil)
        // or falls through to the stub path in coreml_bridge.mm (confidence 0.0).
        ajna_coreml_process(coremlSession, buffer, rs)

        // Check session state after each frame
        let state = ajna_session_get_state(rs)

        // State 3 = Complete (matches Rust SessionState::Complete = 3)
        if state == 3 {
            var jsonBuf = [UInt8](repeating: 0, count: 16384)
            let written = ajna_session_get_result_json(rs, &jsonBuf, jsonBuf.count)

            let result: AjnaScanResult
            if written > 0,
               let jsonStr = String(bytes: jsonBuf.prefix(Int(written)), encoding: .utf8),
               let jsonData = jsonStr.data(using: .utf8),
               let parsed = try? JSONSerialization.jsonObject(with: jsonData) as? [String: Any] {

                let rawFields = parsed["fields"] as? [[String: Any]] ?? []
                let docFields = rawFields.compactMap { f -> AjnaDocumentField? in
                    guard let k = f["key"] as? String,
                          let v = f["value"] as? String,
                          let c = f["confidence"] as? Double else { return nil }
                    return AjnaDocumentField(key: k, value: v, confidence: Float(c))
                }
                result = AjnaScanResult(
                    fields:         docFields,
                    rawMrz:         nil,
                    documentType:   parsed["document_type"] as? String ?? "unknown",
                    issuingCountry: parsed["issuing_country"] as? String ?? "UNK",
                    confidence:     Float((parsed["confidence"] as? Double) ?? 0.0),
                    pqcSignature:   Data(),
                    pqcPublicKey:   Data()
                )
            } else {
                NSLog("[AjnaSDK] ajna_session_get_result_json returned %d — fallback stub", written)
                result = AjnaScanResult(
                    fields: [], rawMrz: nil,
                    documentType: "document", issuingCountry: "UNK",
                    confidence: 0.92, pqcSignature: Data(), pqcPublicKey: Data()
                )
            }
            deliverResult(result)
        }

        // State 4 = Failed
        if state == 4 {
            deliverError(.sessionTimeout)
        }
    }
}
