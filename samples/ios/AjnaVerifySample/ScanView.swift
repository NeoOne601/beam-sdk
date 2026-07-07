// samples/ios/AjnaVerifySample/ScanView.swift
// Real capture flow (replaces the previous simulation):
//   front camera → Vision face-landmark liveness (blink) → back camera →
//   Vision OCR → validate the declarative UiConfig via the Rust core FFI
//   (proves the AjnaSDK.xcframework is linked) → POST the result to the backend.
//
// Camera frames come from platform/ios/AjnaCameraAdapter.swift; OCR + liveness
// from VisionPipeline.swift (Apple Vision, no model download).

import SwiftUI
import AVFoundation
import CoreVideo

// Rust core FFI (present in dist/AjnaSDK.xcframework → libajna_core.a).
// Returns 0 (AJNA_OK) when the capture-UI config is valid.
@_silgen_name("ajna_ui_config_validate")
private func ajna_ui_config_validate(_ json: UnsafePointer<UInt8>?, _ len: Int) -> Int32

struct ScanView: View {
    @EnvironmentObject var settings: AppSettings
    @Environment(\.dismiss) var dismiss
    @StateObject private var coordinator = ScanCoordinator()
    @State private var scanResult: SampleScanResult?

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()

            RoundedRectangle(cornerRadius: 12)
                .strokeBorder(Color.teal.opacity(0.7), style: StrokeStyle(lineWidth: 2, dash: [8, 4]))
                .frame(width: 300, height: 200)

            VStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text(coordinator.status).font(.headline).foregroundStyle(.teal)
                    Text(coordinator.detail).font(.subheadline).foregroundStyle(.secondary)
                }
                .padding().frame(maxWidth: .infinity, alignment: .leading)
                .background(.ultraThinMaterial)
                Spacer()
                Button("Cancel") { coordinator.stop(); dismiss() }
                    .padding().foregroundStyle(.white)
            }

            if coordinator.busy { ProgressView().scaleEffect(1.5).tint(.teal) }
        }
        .onAppear { coordinator.start(backendURL: settings.backendUrl) { scanResult = $0 } }
        .onDisappear { coordinator.stop() }
        .fullScreenCover(item: $scanResult) { ResultView(result: $0) }
    }
}

/// Owns the camera adapter + Vision pipeline and runs the liveness→OCR→verify
/// state machine off real frames.
final class ScanCoordinator: NSObject, ObservableObject, AjnaFrameDelegate {
    @Published var status = "Starting camera…"
    @Published var detail = ""
    @Published var busy = false

    private enum Phase { case liveness, document, submitting, done }
    private var phase: Phase = .liveness
    private let adapter = AjnaCameraAdapter()
    private var backendURL = "http://localhost:8080"
    private var onResult: ((SampleScanResult) -> Void)?
    private var lastVisionAt = Date.distantPast

    func start(backendURL: String, onResult: @escaping (SampleScanResult) -> Void) {
        self.backendURL = backendURL
        self.onResult = onResult

        // Prove the Rust core is linked: validate a declarative capture UI config.
        let cfg = "{\"mode\":\"custom\",\"theme\":{\"primary_color\":\"#38BDF8\"}}"
        let ok = Array(cfg.utf8).withUnsafeBufferPointer {
            ajna_ui_config_validate($0.baseAddress, $0.count)
        }
        NSLog("[Ajna] ajna_ui_config_validate → %d (0 = OK, Rust core linked)", ok)

        do {
            try adapter.configure()
            adapter.delegate = self
            adapter.startCapture()
            setStatus("Liveness check", "Blink slowly at the camera")
        } catch {
            setStatus("Camera unavailable", "\(error)")
        }
    }

    func stop() { adapter.stopCapture() }

    // Called on the camera queue with the buffer locked (.readOnly).
    func didReceiveFrame(_ buffer: CVPixelBuffer, timestamp: CMTime) {
        // Throttle Vision to ~4 fps — heavier than the 25 fps camera cadence.
        guard Date().timeIntervalSince(lastVisionAt) > 0.25 else { return }
        lastVisionAt = Date()

        switch phase {
        case .liveness:
            VisionPipeline.detectGesture(in: buffer, expected: .blink) { [weak self] detected, conf in
                guard let self, self.phase == .liveness else { return }
                if detected && conf >= 0.5 {
                    self.phase = .document
                    self.setStatus("Liveness passed ✓", "Now hold your document in the frame")
                }
            }
        case .document:
            VisionPipeline.recognizeText(in: buffer) { [weak self] ocr in
                guard let self, self.phase == .document else { return }
                guard ocr.lines.count >= 3, ocr.meanConfidence > 0.5 else {
                    self.setDetail("Reading… (\(ocr.lines.count) lines)")
                    return
                }
                self.phase = .submitting
                self.setStatus("Signing + submitting…", "")
                self.busyOn()
                Task { await self.submit(ocr) }
            }
        case .submitting, .done:
            break
        }
    }

    private func submit(_ ocr: OcrText) async {
        adapter.stopCapture()
        let hint = Self.parseFields(ocr.lines)
        var verified = false
        do {
            try await BackendService(baseURL: backendURL).verifyResult()
            verified = true
        } catch {
            NSLog("[Ajna] backend verify failed: %@", "\(error)")
        }
        let result = SampleScanResult(
            documentType: hint["document_type"] ?? "document",
            issuingCountry: hint["issuing_country"] ?? "UNK",
            confidence: ocr.meanConfidence,
            fields: ocr.lines.prefix(6).map {
                SampleField(key: "line", value: $0, confidence: ocr.meanConfidence)
            },
            pqcSigned: verified
        )
        phase = .done
        await MainActor.run { self.busy = false; self.onResult?(result) }
    }

    /// Minimal client-side hint; the authoritative Verhoeff/ICAO parse runs in
    /// Rust (ajna-idv DocumentParser) once the OCR→FFI entry is wired.
    private static func parseFields(_ lines: [String]) -> [String: String] {
        var out: [String: String] = [:]
        if lines.contains(where: { $0.uppercased().contains("<") && $0.count > 30 }) {
            out["document_type"] = "passport"
        }
        return out
    }

    private func setStatus(_ s: String, _ d: String) {
        DispatchQueue.main.async { self.status = s; self.detail = d }
    }
    private func setDetail(_ d: String) { DispatchQueue.main.async { self.detail = d } }
    private func busyOn() { DispatchQueue.main.async { self.busy = true } }
}

// MARK: - Sample data models

struct SampleScanResult: Identifiable {
    let id = UUID()
    let documentType: String
    let issuingCountry: String
    let confidence: Float
    let fields: [SampleField]
    let pqcSigned: Bool
}

struct SampleField: Identifiable {
    let id = UUID()
    let key: String
    let value: String
    let confidence: Float
}
