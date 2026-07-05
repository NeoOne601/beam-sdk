// platform/ios/AjnaCameraAdapter.swift
// AVFoundation camera adapter for Ajna SDK.
// Zero-copy CVPixelBuffer delivery to C++ CoreML bridge.
// Lock semantics: CVPixelBufferLockBaseAddress / UnlockBaseAddress
// wrap every native core call — never hold locks across frames.

import AVFoundation
import CoreVideo

/// Delegate receives zero-copy CVPixelBuffer references.
/// MUST call `finishWithFrame()` before returning to release the lock.
public protocol AjnaFrameDelegate: AnyObject {
    func didReceiveFrame(_ buffer: CVPixelBuffer, timestamp: CMTime)
}

public final class AjnaCameraAdapter: NSObject {

    private var captureSession:     AVCaptureSession?
    private var videoOutput:        AVCaptureVideoDataOutput?
    private let sessionQueue =      DispatchQueue(label: "com.ajna.sdk.camera",
                                                  qos: .userInteractive)
    public weak var delegate:       AjnaFrameDelegate?

    // MARK: – Setup

    public func configure() throws {
        let session = AVCaptureSession()
        // 1920x1080 — balances quality and inference latency.
        session.sessionPreset = .hd1920x1080

        // Back camera — prefer wide angle (focal length 1.0x)
        guard let device = bestBackCamera() else {
            throw AjnaError.noCameraAvailable
        }
        let input = try AVCaptureDeviceInput(device: device)
        session.addInput(input)

        // Lock 25fps to match the Rust pipeline's quality gate cadence
        try device.lockForConfiguration()
        device.activeVideoMinFrameDuration = CMTimeMake(value: 1, timescale: 25)
        device.activeVideoMaxFrameDuration = CMTimeMake(value: 1, timescale: 25)
        // Continuous auto-focus optimised for documents (macro-ish distance ~20–40cm)
        if device.isFocusModeSupported(.continuousAutoFocus) {
            device.focusMode = .continuousAutoFocus
        }
        device.unlockForConfiguration()

        // Configure video output to deliver NV12 (kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange)
        // This is the native ISP output format — no conversion, no copy.
        let output = AVCaptureVideoDataOutput()
        output.videoSettings = [
            kCVPixelBufferPixelFormatTypeKey as String:
                kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange
        ]
        // alwaysDiscardsLateVideoFrames: true — if the pipeline is busy,
        // drop the frame rather than queuing it. Quality gates will reject
        // stale frames anyway.
        output.alwaysDiscardsLateVideoFrames = true
        output.setSampleBufferDelegate(self, queue: sessionQueue)

        session.addOutput(output)

        // Keep video oriented correctly (portrait for document scanning)
        if let connection = output.connection(with: .video) {
            if connection.isVideoRotationAngleSupported(90) {
                connection.videoRotationAngle = 90
            }
        }

        self.captureSession = session
        self.videoOutput    = output
    }

    public func startCapture() {
        sessionQueue.async { self.captureSession?.startRunning() }
    }

    public func stopCapture() {
        sessionQueue.async { self.captureSession?.stopRunning() }
    }

    // MARK: – Camera selection

    /// Prefer the main wide-angle camera (focal length multiplier ≈ 1.0).
    /// Reject ultra-wide (multiplier < 0.7) — too much distortion for OCR.
    private func bestBackCamera() -> AVCaptureDevice? {
        let discovery = AVCaptureDevice.DiscoverySession(
            deviceTypes: [.builtInWideAngleCamera, .builtInDualCamera],
            mediaType:   .video,
            position:    .back
        )
        return discovery.devices.first
    }
}

// MARK: – AVCaptureVideoDataOutputSampleBufferDelegate

extension AjnaCameraAdapter: AVCaptureVideoDataOutputSampleBufferDelegate {

    public func captureOutput(
        _ output:        AVCaptureOutput,
        didOutput sample: CMSampleBuffer,
        from connection: AVCaptureConnection)
    {
        guard let pixelBuffer = CMSampleBufferGetImageBuffer(sample) else { return }
        let timestamp = CMSampleBufferGetPresentationTimeStamp(sample)

        // Lock the CVPixelBuffer for base address access.
        // The Rust + C++ layers will access raw pointers inside this lock.
        CVPixelBufferLockBaseAddress(pixelBuffer, .readOnly)
        delegate?.didReceiveFrame(pixelBuffer, timestamp: timestamp)
        CVPixelBufferUnlockBaseAddress(pixelBuffer, .readOnly)
    }

    public func captureOutput(
        _ output: AVCaptureOutput,
        didDrop sample: CMSampleBuffer,
        from connection: AVCaptureConnection)
    {
        // Dropped frames are expected during heavy inference. Log for diagnostics.
        var reason: CMAttachmentMode = 0
        CMGetAttachment(sample,
            key: kCMSampleBufferAttachmentKey_DroppedFrameReason,
            attachmentModeOut: &reason)
    }
}

// MARK: – Error type
public enum AjnaError: Error {
    case noCameraAvailable
    case configurationFailed(String)
}
