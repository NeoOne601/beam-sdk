// samples/ios/AjnaVerifySample/VisionPipeline.swift
// On-device OCR + liveness using Apple's Vision framework — no external model
// downloads (criterion 1b). VNRecognizeTextRequest reads document text;
// VNDetectFaceLandmarksRequest drives the blink/turn liveness challenge that
// feeds the Ajna liveness FSM. All processing is local; nothing leaves the
// device except the final signed payload the app POSTs to the backend.

import Foundation
import Vision
import CoreVideo

/// Text lines recognized off one document frame, mirroring the Rust
/// `OcrText` the engine's DocumentParser consumes (Verhoeff/ICAO validation).
struct OcrText {
    let lines: [String]
    let meanConfidence: Float
}

/// A single liveness observation derived from face landmarks.
enum LivenessGesture: String { case blink, turnLeft, turnRight, smile, none }

enum VisionPipeline {

    // MARK: Document OCR (Apple Vision, on-device)

    /// Run text recognition over a document pixel buffer. Returns the recognized
    /// lines + mean confidence — the exact shape the Rust OCR parser expects.
    static func recognizeText(in buffer: CVPixelBuffer,
                              completion: @escaping (OcrText) -> Void) {
        let request = VNRecognizeTextRequest { req, _ in
            let obs = (req.results as? [VNRecognizedTextObservation]) ?? []
            var lines: [String] = []
            var confSum: Float = 0
            for o in obs {
                guard let top = o.topCandidates(1).first else { continue }
                lines.append(top.string)
                confSum += top.confidence
            }
            let mean = obs.isEmpty ? 0 : confSum / Float(obs.count)
            completion(OcrText(lines: lines, meanConfidence: mean))
        }
        // .accurate + language correction: best for MRZ / printed IDs.
        request.recognitionLevel = .accurate
        request.usesLanguageCorrection = false   // MRZ is not natural language
        request.recognitionLanguages = ["en-US"]

        let handler = VNImageRequestHandler(cvPixelBuffer: buffer, orientation: .up)
        try? handler.perform([request])
    }

    // MARK: Liveness (face landmarks, on-device)

    /// Detect a gesture from one front-camera frame. Blink is derived from the
    /// eye-aspect-ratio of the landmark points; head turn from the yaw of the
    /// face bounding box + nose position. Feeds the Ajna liveness FSM.
    static func detectGesture(in buffer: CVPixelBuffer,
                              expected: LivenessGesture,
                              completion: @escaping (_ detected: Bool, _ confidence: Float) -> Void) {
        let request = VNDetectFaceLandmarksRequest { req, _ in
            guard let face = (req.results as? [VNFaceObservation])?.first,
                  let landmarks = face.landmarks else {
                completion(false, 0); return
            }
            switch expected {
            case .blink:
                let ear = eyeAspectRatio(landmarks)
                // Eyes closed ⇒ low EAR. Threshold tuned for Vision's normalized points.
                let closed = ear < 0.15
                completion(closed, closed ? min(1.0, (0.15 - ear) / 0.15 + 0.5) : 0)
            case .turnLeft, .turnRight:
                // yaw is provided directly by Vision on modern iOS.
                let yaw = face.yaw?.floatValue ?? 0        // radians, +left / -right
                let want: Float = expected == .turnLeft ? 0.35 : -0.35
                let hit = (want > 0 && yaw >= want) || (want < 0 && yaw <= want)
                completion(hit, hit ? min(1.0, abs(yaw) / 0.6 + 0.4) : 0)
            case .smile, .none:
                completion(false, 0)
            }
        }
        let handler = VNImageRequestHandler(cvPixelBuffer: buffer, orientation: .leftMirrored)
        try? handler.perform([request])
    }

    /// Eye-aspect-ratio averaged over both eyes: vertical opening / horizontal width.
    private static func eyeAspectRatio(_ lm: VNFaceLandmarks2D) -> Float {
        func ear(_ region: VNFaceLandmarkRegion2D?) -> Float {
            guard let pts = region?.normalizedPoints, pts.count >= 4 else { return 1.0 }
            let xs = pts.map { Float($0.x) }, ys = pts.map { Float($0.y) }
            let width = (xs.max()! - xs.min()!)
            let height = (ys.max()! - ys.min()!)
            return width <= 0 ? 1.0 : height / width
        }
        let l = ear(lm.leftEye), r = ear(lm.rightEye)
        return (l + r) / 2
    }
}
