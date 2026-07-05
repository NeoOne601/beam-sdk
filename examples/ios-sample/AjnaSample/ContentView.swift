import SwiftUI

/// Single-screen demo of the full edge pipeline. Heavy lifting is in the Rust
/// core + pillar crates; this view is glue: UiConfig overlay → liveness → scan
/// → ML-DSA-65 sign (in Rust) → POST to backend.
struct ContentView: View {
    @State private var stage: Stage = .liveness
    @State private var status = "Perform the liveness challenge"

    // The same declarative config the dashboard UI Customizer exports.
    private let ui = UiConfig(
        primaryColor: Color(red: 0.22, green: 0.74, blue: 0.97),
        overlayShape: .roundedRect,
        guidePrompt: "Align your document"
    )

    var body: some View {
        VStack(spacing: 16) {
            Text("Ajna Verify").font(.title2).bold()

            // AVFoundation preview goes here in a full build; the overlay is
            // drawn on top and is the visible product of the UiConfig.
            ZStack {
                Rectangle().fill(.black.opacity(0.85))
                CaptureOverlay(ui: ui)
                VStack { Spacer(); Text(status).padding(8) }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            Button(stage == .done ? "Restart" : "Continue", action: advance)
                .buttonStyle(.borderedProminent)
        }
        .padding()
        .onAppear {
            precondition(AjnaBridge.validateUiConfig(ui.json), "UiConfig rejected by ajna-core")
        }
        .preferredColorScheme(.dark)
    }

    private func advance() {
        switch stage {
        case .liveness:
            // LivenessController streams Vision face landmarks → the
            // ajna-vision FSM (blink / turn / smile).
            status = "Liveness passed — now scan your document"
            stage = .document
        case .document:
            status = "Signing + submitting…"
            Task {
                do {
                    let txId = try await captureSignSubmit()
                    status = "Verified ✓  (tx \(txId))"
                } catch {
                    status = "Failed: \(error.localizedDescription)"
                }
                stage = .done
            }
        case .done:
            stage = .liveness
            status = "Perform the liveness challenge"
        }
    }

    /// Capture → OCR parse → ML-DSA-65 sign in Rust → POST to backend.
    private func captureSignSubmit() async throws -> String {
        guard let handle = AjnaBridge.createSession() else { throw AjnaError.session }
        defer { AjnaBridge.destroy(handle) }

        // Native OCR (Tesseract/Paddle) fills fields on device; the Rust side
        // signs with ML-DSA-65 via ajna-idv DocumentParser.scan_and_sign.
        guard let json = AjnaBridge.resultJSON(handle) else { throw AjnaError.noResult }

        let nonce = try await BackendClient.shared.fetchNonce()
        return try await BackendClient.shared.verify(signedResultJSON: json, nonce: nonce)
    }
}

private struct CaptureOverlay: View {
    let ui: UiConfig
    var body: some View {
        GeometryReader { geo in
            let w = geo.size.width * 0.8
            let h = geo.size.height * 0.5
            let shape = RoundedRectangle(cornerRadius: ui.overlayShape == .oval ? h / 2 : 16)
            shape
                .stroke(ui.primaryColor, lineWidth: 3)
                .frame(width: w, height: h)
                .position(x: geo.size.width / 2, y: geo.size.height / 2)
        }
    }
}

enum Stage { case liveness, document, done }
enum AjnaError: Error { case session, noResult }

enum OverlayShape { case roundedRect, oval }

struct UiConfig {
    let primaryColor: Color
    let overlayShape: OverlayShape
    let guidePrompt: String

    /// Serialize to the same schema `ajna_ui_config_validate` accepts.
    var json: String {
        """
        {"mode":"custom","overlay":{"shape":"\(overlayShape == .oval ? "oval" : "rounded_rect")"},\
        "strings":{"scan_prompt":"\(guidePrompt)"}}
        """
    }
}
