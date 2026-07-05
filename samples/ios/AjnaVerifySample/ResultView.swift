// samples/ios/AjnaVerifySample/ResultView.swift
// Displays scan results with document fields and backend verification.

import SwiftUI

struct ResultView: View {
    @Environment(\.dismiss) var dismiss
    @EnvironmentObject var settings: AppSettings
    let result: SampleScanResult

    @State private var verificationStatus: VerificationStatus = .idle

    enum VerificationStatus {
        case idle, loading, success, failed(String)
    }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    // Header
                    HStack {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(result.documentType.uppercased())
                                .font(.title2)
                                .fontWeight(.bold)

                            Text(result.issuingCountry)
                                .font(.subheadline)
                                .foregroundStyle(.secondary)
                        }

                        Spacer()

                        // Confidence gauge
                        ZStack {
                            Circle()
                                .stroke(.gray.opacity(0.2), lineWidth: 6)

                            Circle()
                                .trim(from: 0, to: CGFloat(result.confidence))
                                .stroke(.teal, style: StrokeStyle(lineWidth: 6, lineCap: .round))
                                .rotationEffect(.degrees(-90))

                            Text("\(Int(result.confidence * 100))%")
                                .font(.caption)
                                .fontWeight(.bold)
                        }
                        .frame(width: 56, height: 56)
                    }

                    // PQC status
                    Label(
                        result.pqcSigned ? "PQC Signed (ML-DSA Level 3)" : "Not signed",
                        systemImage: result.pqcSigned ? "checkmark.shield" : "xmark.shield"
                    )
                    .font(.subheadline)
                    .foregroundStyle(result.pqcSigned ? .green : .red)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(result.pqcSigned ? .green.opacity(0.1) : .red.opacity(0.1))
                    .clipShape(Capsule())

                    Divider()

                    // Document fields
                    ForEach(result.fields) { field in
                        VStack(alignment: .leading, spacing: 4) {
                            Text(field.key.replacingOccurrences(of: "_", with: " ").uppercased())
                                .font(.caption)
                                .foregroundStyle(.secondary)

                            HStack {
                                Text(field.value)
                                    .font(.body)
                                    .fontWeight(.medium)

                                Spacer()

                                Text("\(Int(field.confidence * 100))%")
                                    .font(.caption2)
                                    .foregroundStyle(.teal)
                            }
                        }
                        .padding(.vertical, 4)
                    }

                    Divider()

                    // Backend verification
                    Button(action: verifyWithBackend) {
                        HStack {
                            switch verificationStatus {
                            case .idle:
                                Label("Verify with Backend", systemImage: "arrow.up.circle")
                            case .loading:
                                ProgressView()
                                    .tint(.white)
                                Text("Verifying...")
                            case .success:
                                Label("Verified", systemImage: "checkmark.circle")
                            case .failed(let msg):
                                Label("Failed: \(msg)", systemImage: "xmark.circle")
                            }
                        }
                        .frame(maxWidth: .infinity)
                        .padding()
                        .background(verificationColor)
                        .foregroundStyle(.white)
                        .fontWeight(.semibold)
                        .clipShape(RoundedRectangle(cornerRadius: 12))
                    }
                    .disabled(verificationStatus is VerificationStatus)

                    Button("New Scan") {
                        dismiss()
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(.gray.opacity(0.2))
                    .foregroundStyle(.secondary)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
                }
                .padding(24)
            }
            .navigationTitle("Scan Result")
            .navigationBarTitleDisplayMode(.inline)
        }
    }

    private var verificationColor: Color {
        switch verificationStatus {
        case .idle: return .purple
        case .loading: return .purple.opacity(0.7)
        case .success: return .green
        case .failed: return .red
        }
    }

    private func verifyWithBackend() {
        verificationStatus = .loading
        Task {
            do {
                let service = BackendService(baseURL: settings.backendUrl)
                try await service.verifyResult()
                verificationStatus = .success
            } catch {
                verificationStatus = .failed(error.localizedDescription)
            }
        }
    }
}
