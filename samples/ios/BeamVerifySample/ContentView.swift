// samples/ios/BeamVerifySample/ContentView.swift
// Main navigation hub for the Beam Verify iOS sample.

import SwiftUI

struct ContentView: View {
    @EnvironmentObject var settings: AppSettings
    @State private var showingScan = false

    var body: some View {
        NavigationStack {
            VStack(spacing: 32) {
                Spacer()

                // Logo and title
                VStack(spacing: 8) {
                    Image(systemName: "doc.viewfinder")
                        .font(.system(size: 64))
                        .foregroundStyle(.purple)

                    Text("Beam Verify")
                        .font(.largeTitle)
                        .fontWeight(.bold)
                        .foregroundStyle(.white)

                    Text("SDK v0.1.0")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }

                // Feature list
                VStack(alignment: .leading, spacing: 12) {
                    FeatureRow(icon: "lock.shield", text: "ML-DSA (FIPS 204) result signing")
                    FeatureRow(icon: "camera.filters", text: "On-device quality gates")
                    FeatureRow(icon: "cpu", text: "CoreML Neural Engine inference")
                    FeatureRow(icon: "key", text: "mlock()-protected key storage")
                }
                .padding()
                .background(.ultraThinMaterial)
                .clipShape(RoundedRectangle(cornerRadius: 16))

                Spacer()

                // Action buttons
                Button(action: { showingScan = true }) {
                    Label("Start Document Scan", systemImage: "doc.viewfinder")
                        .frame(maxWidth: .infinity)
                        .padding()
                        .background(.purple)
                        .foregroundStyle(.white)
                        .fontWeight(.semibold)
                        .clipShape(RoundedRectangle(cornerRadius: 12))
                }

                NavigationLink(destination: SettingsView()) {
                    Label("Settings", systemImage: "gear")
                        .frame(maxWidth: .infinity)
                        .padding()
                        .background(.gray.opacity(0.2))
                        .foregroundStyle(.secondary)
                        .clipShape(RoundedRectangle(cornerRadius: 12))
                }
            }
            .padding(24)
            .background(Color(uiColor: .systemBackground))
            .fullScreenCover(isPresented: $showingScan) {
                ScanView()
            }
        }
    }
}

struct FeatureRow: View {
    let icon: String
    let text: String

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: icon)
                .foregroundStyle(.purple)
                .frame(width: 24)
            Text(text)
                .font(.subheadline)
                .foregroundStyle(.primary)
        }
    }
}
