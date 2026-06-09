// samples/ios/BeamVerifySample/BackendService.swift
// HTTP client for Beam Verify backend (nonce + verify flow).

import Foundation

struct BackendService {
    let baseURL: String

    struct NonceResponse: Codable {
        let nonce: String
        let expires_at: String
        let session_id: String
    }

    struct VerifyResponse: Codable {
        let verified: Bool
        let verification_id: String
    }

    /// Perform the nonce → verify flow.
    func verifyResult() async throws {
        let sessionId = UUID().uuidString

        // Step 1: Get nonce
        let nonceURL = URL(string: "\(baseURL)/v1/nonce")!
        var nonceReq = URLRequest(url: nonceURL)
        nonceReq.httpMethod = "POST"
        nonceReq.setValue("application/json", forHTTPHeaderField: "Content-Type")
        nonceReq.httpBody = try JSONEncoder().encode(["session_id": sessionId])

        let (nonceData, _) = try await URLSession.shared.data(for: nonceReq)
        let nonceResp = try JSONDecoder().decode(NonceResponse.self, from: nonceData)

        // Step 2: Verify
        let verifyURL = URL(string: "\(baseURL)/v1/verify")!
        var verifyReq = URLRequest(url: verifyURL)
        verifyReq.httpMethod = "POST"
        verifyReq.setValue("application/json", forHTTPHeaderField: "Content-Type")

        let verifyBody: [String: Any] = [
            "session_id": sessionId,
            "nonce": nonceResp.nonce,
            "scan_result": [
                "fields": [] as [[String: Any]],
                "document_type": "passport",
                "issuing_country": "USA",
                "confidence": 0.97,
                "pqc_signature": "",
                "pqc_public_key": "",
            ] as [String: Any],
        ]
        verifyReq.httpBody = try JSONSerialization.data(withJSONObject: verifyBody)

        let (_, verifyResp) = try await URLSession.shared.data(for: verifyReq)
        guard let httpResp = verifyResp as? HTTPURLResponse,
              httpResp.statusCode == 200 else {
            throw URLError(.badServerResponse)
        }
    }
}
