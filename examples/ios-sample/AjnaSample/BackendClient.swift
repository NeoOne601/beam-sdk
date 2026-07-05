import Foundation

/// Minimal client for the Ajna verify backend: fetch a nonce, then POST the
/// ML-DSA-65-signed scan result to `/v1/verify`.
struct BackendClient {
    static let shared = BackendClient()

    // Override for a real device run.
    private let baseURL = URL(string: "https://ajna-verify.fly.dev")!
    private let apiKey = "ajna_live_sk_demo_0000"

    func fetchNonce() async throws -> String {
        let json = try await post(path: "/v1/nonce", body: Data("{}".utf8))
        return json["nonce"] as? String ?? ""
    }

    /// Returns the backend's verification_id on success.
    func verify(signedResultJSON: String, nonce: String) async throws -> String {
        // Attach the nonce the backend just issued to the signed payload.
        var payload = (try? JSONSerialization.jsonObject(with: Data(signedResultJSON.utf8))
            as? [String: Any]) ?? [:]
        payload["nonce"] = nonce
        let body = try JSONSerialization.data(withJSONObject: payload)
        let json = try await post(path: "/v1/verify", body: body)
        return json["verification_id"] as? String ?? "unknown"
    }

    private func post(path: String, body: Data) async throws -> [String: Any] {
        var req = URLRequest(url: baseURL.appendingPathComponent(path))
        req.httpMethod = "POST"
        req.httpBody = body
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.setValue(apiKey, forHTTPHeaderField: "X-Api-Key")
        let (data, _) = try await URLSession.shared.data(for: req)
        return (try JSONSerialization.jsonObject(with: data) as? [String: Any]) ?? [:]
    }
}
