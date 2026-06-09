// samples/ios/BeamVerifySample/App.swift
// Beam Verify iOS Sample App — SwiftUI entry point.

import SwiftUI

@main
struct BeamVerifySampleApp: App {
    @StateObject private var settings = AppSettings()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(settings)
                .preferredColorScheme(.dark)
        }
    }
}

/// Persisted app settings.
class AppSettings: ObservableObject {
    @Published var backendUrl: String {
        didSet { UserDefaults.standard.set(backendUrl, forKey: "backendUrl") }
    }
    @Published var pqcSigningEnabled: Bool {
        didSet { UserDefaults.standard.set(pqcSigningEnabled, forKey: "pqcSigning") }
    }
    @Published var includeRawMrz: Bool {
        didSet { UserDefaults.standard.set(includeRawMrz, forKey: "includeRawMrz") }
    }

    init() {
        self.backendUrl = UserDefaults.standard.string(forKey: "backendUrl") ?? "http://localhost:8080"
        self.pqcSigningEnabled = UserDefaults.standard.object(forKey: "pqcSigning") as? Bool ?? true
        self.includeRawMrz = UserDefaults.standard.object(forKey: "includeRawMrz") as? Bool ?? false
    }
}
