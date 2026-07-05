// samples/ios/AjnaVerifySample/SettingsView.swift
// Settings screen for configuring the Ajna Verify sample app.

import SwiftUI

struct SettingsView: View {
    @EnvironmentObject var settings: AppSettings

    var body: some View {
        Form {
            Section("Backend") {
                TextField("Backend URL", text: $settings.backendUrl)
                    .textContentType(.URL)
                    .autocapitalization(.none)
                    .disableAutocorrection(true)
            }

            Section("SDK Options") {
                Toggle("PQC Signing (ML-DSA Level 3)", isOn: $settings.pqcSigningEnabled)
                Toggle("Include Raw MRZ", isOn: $settings.includeRawMrz)
            }

            Section("About") {
                LabeledContent("SDK Version", value: "0.1.0")
                LabeledContent("Signing Algorithm", value: "ML-DSA Level 3")
                LabeledContent("Key Storage", value: "mlock()-protected")
                LabeledContent("Inference Engine", value: "CoreML")
            }
        }
        .navigationTitle("Settings")
    }
}
