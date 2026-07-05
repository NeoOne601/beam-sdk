// Ajna tool implementations.
//
// Two execution classes:
//   - Local:   posture evaluation and face verification run the pillar
//              crates in-process and sign output with the server's
//              ajna-crypto signer (Ed25519 or ML-DSA-65).
//   - Backend: document verification and audit queries proxy to the Ajna
//              verify backend over HTTPS, authenticated with the tenant
//              API key. Without AJNA_BACKEND_URL these return a tool error
//              that tells the agent exactly what is missing.

use crate::server::ServerState;
use ajna_intel::DeviceIndicators;
use ajna_vision::{
    verify_face, ChallengeObservation, FaceEmbedding, LivenessConfig, LivenessSession,
    DEFAULT_MATCH_THRESHOLD,
};
use serde_json::{json, Value};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub enum ToolError {
    UnknownTool,
    Execution(String),
}

impl ToolError {
    fn execution(message: impl Into<String>) -> Self {
        Self::Execution(message.into())
    }
}

/// MCP tool descriptors, in the order agents see them.
pub fn tool_definitions() -> Value {
    json!([
        {
            "name": "ajna_evaluate_device_posture",
            "description": "Evaluate raw device facts (artifact paths, loaded libraries, \
                build properties, debugger state) into a risk-scored, cryptographically \
                signed posture report. Verdicts: trusted | suspicious | compromised.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "indicators": {
                        "type": "object",
                        "description": "DeviceIndicators: platform (ios|android|web), \
                            present_paths, loaded_libraries, build_properties, \
                            debugger_attached, selinux_enforcing",
                        "properties": {
                            "platform": { "type": "string", "enum": ["ios", "android", "web"] },
                            "present_paths": { "type": "array", "items": { "type": "string" } },
                            "loaded_libraries": { "type": "array", "items": { "type": "string" } },
                            "build_properties": { "type": "object" },
                            "debugger_attached": { "type": "boolean" },
                            "selinux_enforcing": { "type": ["boolean", "null"] }
                        },
                        "required": ["platform"]
                    }
                },
                "required": ["indicators"]
            }
        },
        {
            "name": "ajna_verify_face",
            "description": "Run liveness challenge validation over gesture observations and \
                compare face embeddings. Returns a signed vision result with verdict \
                verified | liveness_failed | face_mismatch.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "observations": {
                        "type": "array",
                        "description": "Ordered gesture observations: challenge \
                            (blink|turn_left|turn_right|smile|nod), confidence, timestamp_us",
                        "items": { "type": "object" }
                    },
                    "probe": { "type": "array", "items": { "type": "number" } },
                    "reference": { "type": "array", "items": { "type": "number" } },
                    "threshold": { "type": "number", "description": "Cosine threshold, default 0.75" }
                },
                "required": ["observations", "probe", "reference"]
            }
        },
        {
            "name": "ajna_verify_document",
            "description": "Verify a signed Ajna IDV scan result against the Ajna backend \
                (signature check, nonce replay protection, country rules). Requires \
                AJNA_BACKEND_URL and AJNA_API_KEY.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "payload": {
                        "type": "object",
                        "description": "The scan result payload exactly as produced by the Ajna SDK"
                    }
                },
                "required": ["payload"]
            }
        },
        {
            "name": "ajna_query_audit_log",
            "description": "Fetch recent entries from the tenant's tamper-evident audit log \
                (SOC2 evidence trail). Requires AJNA_BACKEND_URL and AJNA_API_KEY.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "minimum": 1, "maximum": 500, "default": 50 }
                }
            }
        }
    ])
}

pub fn call_tool(state: &ServerState, name: &str, arguments: &Value) -> Result<Value, ToolError> {
    match name {
        "ajna_evaluate_device_posture" => evaluate_device_posture(state, arguments),
        "ajna_verify_face" => verify_face_tool(state, arguments),
        "ajna_verify_document" => backend_post(state, "/v1/verify", arguments.get("payload")),
        "ajna_query_audit_log" => query_audit_log(state, arguments),
        _ => Err(ToolError::UnknownTool),
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

fn evaluate_device_posture(state: &ServerState, arguments: &Value) -> Result<Value, ToolError> {
    let indicators_value = arguments
        .get("indicators")
        .cloned()
        .ok_or_else(|| ToolError::execution("missing required argument: indicators"))?;
    let indicators: DeviceIndicators = serde_json::from_value(indicators_value)
        .map_err(|e| ToolError::execution(format!("invalid indicators: {e}")))?;

    let report = ajna_intel::evaluate(&indicators, now_unix());
    let signed = report
        .sign(state.signer.as_ref())
        .map_err(|e| ToolError::execution(format!("signing failed: {e}")))?;
    serde_json::to_value(&signed)
        .map_err(|e| ToolError::execution(format!("serialization failed: {e}")))
}

fn verify_face_tool(state: &ServerState, arguments: &Value) -> Result<Value, ToolError> {
    let observations: Vec<ChallengeObservation> =
        serde_json::from_value(arguments.get("observations").cloned().unwrap_or(json!([])))
            .map_err(|e| ToolError::execution(format!("invalid observations: {e}")))?;
    if observations.is_empty() {
        return Err(ToolError::execution("at least one observation is required"));
    }

    let probe = parse_embedding(arguments, "probe")?;
    let reference = parse_embedding(arguments, "reference")?;
    let threshold = arguments
        .get("threshold")
        .and_then(Value::as_f64)
        .map(|t| t as f32)
        .unwrap_or(DEFAULT_MATCH_THRESHOLD);

    let mut session = LivenessSession::new(LivenessConfig::default())
        .map_err(|e| ToolError::execution(format!("invalid liveness config: {e:?}")))?;
    session.start(observations[0].timestamp_us.saturating_sub(1));
    for observation in observations {
        session.submit(observation);
    }

    let result = verify_face(&session, &probe, &reference, threshold, now_unix())
        .map_err(|e| ToolError::execution(format!("face comparison failed: {e}")))?;
    let signed = result
        .sign(state.signer.as_ref())
        .map_err(|e| ToolError::execution(format!("signing failed: {e}")))?;
    serde_json::to_value(&signed)
        .map_err(|e| ToolError::execution(format!("serialization failed: {e}")))
}

fn parse_embedding(arguments: &Value, key: &str) -> Result<FaceEmbedding, ToolError> {
    let values: Vec<f32> = serde_json::from_value(arguments.get(key).cloned().unwrap_or(json!([])))
        .map_err(|e| ToolError::execution(format!("invalid {key} embedding: {e}")))?;
    FaceEmbedding::new(values)
        .map_err(|e| ToolError::execution(format!("invalid {key} embedding: {e}")))
}

fn query_audit_log(state: &ServerState, arguments: &Value) -> Result<Value, ToolError> {
    let limit = arguments.get("limit").and_then(Value::as_u64).unwrap_or(50);
    backend_get(state, &format!("/v1/audit?limit={limit}"))
}

// ─── Backend proxy helpers ───────────────────────────────────────────────────

fn backend_config(state: &ServerState) -> Result<(&str, &str), ToolError> {
    let url = state.backend_url.as_deref().ok_or_else(|| {
        ToolError::execution(
            "Ajna backend not configured: set AJNA_BACKEND_URL (and AJNA_API_KEY) \
             in the MCP server environment to enable this tool",
        )
    })?;
    let key = state.api_key.as_deref().ok_or_else(|| {
        ToolError::execution("AJNA_API_KEY is not set in the MCP server environment")
    })?;
    Ok((url, key))
}

fn backend_post(state: &ServerState, path: &str, payload: Option<&Value>) -> Result<Value, ToolError> {
    let (base, key) = backend_config(state)?;
    let payload = payload.ok_or_else(|| ToolError::execution("missing required argument: payload"))?;
    let client = http_client()?;
    let response = client
        .post(format!("{}{path}", base.trim_end_matches('/')))
        .header("X-Api-Key", key)
        .json(payload)
        .send()
        .map_err(|e| ToolError::execution(format!("backend request failed: {e}")))?;
    read_backend_response(response)
}

fn backend_get(state: &ServerState, path: &str) -> Result<Value, ToolError> {
    let (base, key) = backend_config(state)?;
    let client = http_client()?;
    let response = client
        .get(format!("{}{path}", base.trim_end_matches('/')))
        .header("X-Api-Key", key)
        .send()
        .map_err(|e| ToolError::execution(format!("backend request failed: {e}")))?;
    read_backend_response(response)
}

fn http_client() -> Result<reqwest::blocking::Client, ToolError> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| ToolError::execution(format!("http client init failed: {e}")))
}

fn read_backend_response(response: reqwest::blocking::Response) -> Result<Value, ToolError> {
    let status = response.status();
    let body: Value = response
        .json()
        .unwrap_or_else(|_| json!({ "error": "backend returned a non-JSON body" }));
    if status.is_success() {
        Ok(body)
    } else {
        Err(ToolError::execution(format!("backend returned {status}: {body}")))
    }
}
