// platform/ios/BeamSDK.swift
// Public Beam SDK interface for iOS.
// BeamScanner owns a BeamCameraAdapter and coordinates with the CoreML bridge.
// All public completion callbacks are dispatched on DispatchQueue.main.

import AVFoundation
import CoreVideo
import Foundation

// MARK: – Configuration

/// Mirror of Rust SessionConfig, exposed to iOS integrators.
public struct BeamScanConfig {
    /// Number of quality-passing frames required before inference triggers.
    /// Default: 3.
    public var minQualityFrames: Int
    /// Session timeout in milliseconds. Default: 30000 (30 seconds).
    public var timeoutMs:         Int
    /// Whether to PQC-sign the result. Set false in dev builds to reduce latency.
    public var pqcSignResult:     Bool

    public init(
        minQualityFrames: Int  = 3,
        timeoutMs:         Int  = 30_000,
        pqcSignResult:     Bool = true
    ) {
        self.minQualityFrames = minQualityFrames
        self.timeoutMs         = timeoutMs
        self.pqcSignResult     = pqcSignResult
    }
}

// MARK: – Result

/// Mirror of Rust ScanResult, Codable for persistence or network serialisation.
public struct BeamScanResult: Codable {
    /// Extracted document fields (e.g. surname, document_number).
    public let fields:          [BeamDocumentField]
    /// Raw MRZ string, if requested in BeamScanConfig.
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
public struct BeamDocumentField: Codable {
    public let key:        String
    public let value:      String
    public let confidence: Float
}

// MARK: – Errors

/// Errors surfaced by BeamScanner.
public enum BeamScannerError: Error {
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

// MARK: – BeamScanner

/// Primary public interface for document scanning on iOS.
///
/// Usage:
/// ```swift
/// let scanner = BeamScanner()
/// try scanner.configure()
/// scanner.startScan(config: BeamScanConfig()) { result in
///     print(result.documentType, result.issuingCountry)
/// }
/// ```
public class BeamScanner: NSObject {

    private var cameraAdapter: BeamCameraAdapter?
    private var completion:    ((Result<BeamScanResult, BeamScannerError>) -> Void)?
    private var scanConfig:    BeamScanConfig = BeamScanConfig()

    // MARK: – Public API

    /// Configure the camera session. Must be called before startScan().
    /// Throws BeamScannerError.noCameraAvailable on failure.
    public func configure() throws {
        let adapter = BeamCameraAdapter()
        do {
            try adapter.configure()
        } catch let e as BeamError {
            switch e {
            case .noCameraAvailable:
                throw BeamScannerError.noCameraAvailable
            case .configurationFailed(let msg):
                throw BeamScannerError.configurationFailed(msg)
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
        config:     BeamScanConfig = BeamScanConfig(),
        completion: @escaping (Result<BeamScanResult, BeamScannerError>) -> Void
    ) {
        self.scanConfig = config
        self.completion = completion
        cameraAdapter?.startCapture()
    }

    /// Stop the active scan and release camera resources.
    public func stopScan() {
        cameraAdapter?.stopCapture()
        completion = nil
    }

    // MARK: – Internal result dispatch

    private func deliverResult(_ result: BeamScanResult) {
        let block = self.completion
        self.completion = nil
        cameraAdapter?.stopCapture()
        DispatchQueue.main.async {
            block?(.success(result))
        }
    }

    private func deliverError(_ error: BeamScannerError) {
        let block = self.completion
        self.completion = nil
        cameraAdapter?.stopCapture()
        DispatchQueue.main.async {
            block?(.failure(error))
        }
    }
}

// MARK: – BeamFrameDelegate

extension BeamScanner: BeamFrameDelegate {

    /// Called by BeamCameraAdapter for each locked CVPixelBuffer.
    /// The buffer is already locked with .readOnly by the adapter.
    /// We must call beam_coreml_process (via @_silgen_name bridge) and return.
    /// The adapter will unlock the buffer immediately after this call returns.
    public func didReceiveFrame(_ buffer: CVPixelBuffer, timestamp: CMTime) {
        // In a full integration, this would:
        // 1. Retrieve or lazily create a beam_coreml_session_t* for the model path.
        // 2. Call beam_coreml_process(session, buffer, rust_session_handle).
        // 3. Check beam_session_get_state(rust_session_handle) for completion.
        //
        // For the SDK core, we synthesise a result to demonstrate the callback chain.
        // Integrators replace this block with the CoreML bridge call.
        //
        // Example real call (requires @_silgen_name or bridging header):
        //   beam_coreml_process(coremlSession, buffer, rustSessionHandle)
        //
        // If the session is complete, build and deliver BeamScanResult:
        let syntheticResult = BeamScanResult(
            fields:         [],
            rawMrz:         nil,
            documentType:   "passport",
            issuingCountry: "UNK",
            confidence:     0.0,
            pqcSignature:   Data(),
            pqcPublicKey:   Data()
        )
        // Stub: deliver after first frame for testability.
        // Replace with real state machine polling in production integration.
        _ = syntheticResult
        // deliverResult(syntheticResult)
    }
}
