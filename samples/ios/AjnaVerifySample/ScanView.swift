// samples/ios/AjnaVerifySample/ScanView.swift
// Camera scanning view with quality gate status overlay.
//
// Architecture note:
//   Camera (Swift AVCaptureSession) → AjnaCameraAdapter → Quality Gates (Rust via C FFI)
//   → CoreML inference (C++ bridge) → ajna_session_push_result (Rust FFI)
//   → PQC signing (Rust) → ScanResult delivered to this view

import SwiftUI
import AVFoundation

struct ScanView: View {
    @Environment(\.dismiss) var dismiss
    @State private var gateStatus = "Initialising camera..."
    @State private var qualityFrames = 0
    @State private var isScanning = true
    @State private var scanResult: SampleScanResult?

    var body: some View {
        ZStack {
            // Camera preview placeholder
            Color.black
                .ignoresSafeArea()

            // Document alignment guide
            RoundedRectangle(cornerRadius: 12)
                .strokeBorder(Color.purple.opacity(0.6), style: StrokeStyle(lineWidth: 2, dash: [8, 4]))
                .frame(width: 300, height: 200)

            // Gate status overlay
            VStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text(gateStatus)
                        .font(.headline)
                        .foregroundStyle(.teal)

                    Text("Quality frames: \(qualityFrames) / 3")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
                .padding()
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(.ultraThinMaterial)

                Spacer()

                // Cancel button
                Button("Cancel") {
                    dismiss()
                }
                .padding()
                .foregroundStyle(.white)
            }

            if !isScanning {
                ProgressView()
                    .scaleEffect(1.5)
                    .tint(.purple)
            }
        }
        .onAppear {
            simulateScanFlow()
        }
        .fullScreenCover(item: $scanResult) { result in
            ResultView(result: result)
        }
    }

    /// Simulates the scan flow for demonstration.
    /// In production, this is driven by real AVCaptureSession frames and Ajna SDK callbacks.
    private func simulateScanFlow() {
        let gates = ["BlurCheck", "ExposureCheck", "MotionCheck", "BoundaryCheck", "Accepted"]

        for (i, gate) in gates.enumerated() {
            DispatchQueue.main.asyncAfter(deadline: .now() + Double(i + 1) * 0.8) {
                gateStatus = "Gate: \(gate)"
                if gate == "Accepted" {
                    qualityFrames = min(i, 3)
                }
            }
        }

        DispatchQueue.main.asyncAfter(deadline: .now() + 5.0) {
            gateStatus = "Running CoreML inference..."
            qualityFrames = 3
        }

        DispatchQueue.main.asyncAfter(deadline: .now() + 6.0) {
            gateStatus = "Signing with ML-DSA Level 3..."
        }

        DispatchQueue.main.asyncAfter(deadline: .now() + 7.0) {
            isScanning = false
            scanResult = SampleScanResult(
                documentType: "passport",
                issuingCountry: "USA",
                confidence: 0.97,
                fields: [
                    .init(key: "surname", value: "SMITH", confidence: 0.98),
                    .init(key: "given_names", value: "JOHN MICHAEL", confidence: 0.96),
                    .init(key: "date_of_birth", value: "1990-05-15", confidence: 0.99),
                    .init(key: "document_number", value: "C01X00T47", confidence: 0.97),
                    .init(key: "expiry_date", value: "2030-05-14", confidence: 0.95),
                ],
                pqcSigned: true
            )
        }
    }
}

// MARK: - Sample data model

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
