// JSON-RPC 2.0 dispatch for the MCP stdio transport.
//
// Pure request-in / response-out: `handle_message` takes one line and
// returns the response line (or None for notifications), so the entire
// protocol surface is unit-testable without touching stdin/stdout.

use crate::tools;
use ajna_crypto::{AjnaSigner, EdDsaSigner, MlDsaSigner};
use serde_json::{json, Value};
use std::sync::Arc;

pub const PROTOCOL_VERSION: &str = "2024-11-05";
pub const SERVER_NAME: &str = "ajna-mcp-server";

// JSON-RPC 2.0 error codes.
const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

/// Server configuration + the signer used for local tool output.
pub struct ServerState {
    pub backend_url: Option<String>,
    pub api_key: Option<String>,
    pub signer: Arc<dyn AjnaSigner>,
}

impl ServerState {
    pub fn from_env() -> Self {
        let signer: Arc<dyn AjnaSigner> =
            match std::env::var("AJNA_MCP_SIGN_ALGO").as_deref() {
                Ok("ml-dsa-65") => Arc::new(MlDsaSigner::new()),
                _ => Arc::new(EdDsaSigner::new()),
            };
        Self {
            backend_url: std::env::var("AJNA_BACKEND_URL").ok(),
            api_key: std::env::var("AJNA_API_KEY").ok(),
            signer,
        }
    }

    /// Test/local constructor with explicit configuration.
    pub fn new(backend_url: Option<String>, api_key: Option<String>) -> Self {
        Self {
            backend_url,
            api_key,
            signer: Arc::new(EdDsaSigner::new()),
        }
    }
}

/// Process one JSON-RPC message line. Returns the serialized response, or
/// `None` when no response is due (notifications).
pub fn handle_message(state: &mut ServerState, line: &str) -> Option<String> {
    let message: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(_) => return Some(error_response(Value::Null, PARSE_ERROR, "parse error")),
    };

    let id = message.get("id").cloned();
    let method = message.get("method").and_then(Value::as_str);

    let Some(method) = method else {
        return Some(error_response(
            id.unwrap_or(Value::Null),
            INVALID_REQUEST,
            "missing method",
        ));
    };

    // Notifications (no id) get no response, whatever the method.
    let Some(id) = id else {
        return None;
    };

    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let response = match method {
        "initialize" => ok_response(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": SERVER_NAME,
                    "version": env!("CARGO_PKG_VERSION"),
                },
            }),
        ),
        "ping" => ok_response(id, json!({})),
        "tools/list" => ok_response(id, json!({ "tools": tools::tool_definitions() })),
        "tools/call" => handle_tool_call(state, id, &params),
        _ => error_response(id, METHOD_NOT_FOUND, &format!("unknown method: {method}")),
    };
    Some(response)
}

fn handle_tool_call(state: &mut ServerState, id: Value, params: &Value) -> String {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return error_response(id, INVALID_PARAMS, "tools/call requires params.name");
    };
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    match tools::call_tool(state, name, &arguments) {
        Ok(output) => ok_response(
            id,
            json!({
                "content": [ { "type": "text", "text": output.to_string() } ],
                "isError": false,
            }),
        ),
        Err(tools::ToolError::UnknownTool) => {
            error_response(id, INVALID_PARAMS, &format!("unknown tool: {name}"))
        }
        // Tool execution failures are results with isError=true per MCP,
        // so agents can read the failure and adapt.
        Err(tools::ToolError::Execution(message)) => ok_response(
            id,
            json!({
                "content": [ { "type": "text", "text": message } ],
                "isError": true,
            }),
        ),
    }
}

fn ok_response(id: Value, result: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

fn error_response(id: Value, code: i64, message: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_state() -> ServerState {
        ServerState::new(None, None)
    }

    fn request(state: &mut ServerState, body: Value) -> Value {
        let line = body.to_string();
        let response = handle_message(state, &line).expect("request must get a response");
        serde_json::from_str(&response).expect("response must be valid JSON")
    }

    #[test]
    fn initialize_handshake_advertises_tools_capability() {
        let mut state = local_state();
        let response = request(
            &mut state,
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
        );
        assert_eq!(response["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(response["result"]["serverInfo"]["name"], SERVER_NAME);
        assert!(response["result"]["capabilities"]["tools"].is_object());
    }

    #[test]
    fn tools_list_exposes_all_four_ajna_tools() {
        let mut state = local_state();
        let response = request(
            &mut state,
            json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
        );
        let tools = response["result"]["tools"].as_array().expect("tools array");
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "ajna_evaluate_device_posture",
                "ajna_verify_face",
                "ajna_verify_document",
                "ajna_query_audit_log",
            ]
        );
        for tool in tools {
            assert!(tool["inputSchema"]["type"].as_str() == Some("object"));
            assert!(tool["description"].as_str().is_some());
        }
    }

    #[test]
    fn notifications_get_no_response() {
        let mut state = local_state();
        let line =
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string();
        assert!(handle_message(&mut state, &line).is_none());
    }

    #[test]
    fn unknown_method_is_method_not_found() {
        let mut state = local_state();
        let response = request(
            &mut state,
            json!({ "jsonrpc": "2.0", "id": 3, "method": "resources/list" }),
        );
        assert_eq!(response["error"]["code"], METHOD_NOT_FOUND);
    }

    #[test]
    fn malformed_json_is_parse_error() {
        let mut state = local_state();
        let response = handle_message(&mut state, "{not json").expect("parse error response");
        let response: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(response["error"]["code"], PARSE_ERROR);
    }

    #[test]
    fn posture_tool_flags_rooted_device() {
        let mut state = local_state();
        let response = request(
            &mut state,
            json!({
                "jsonrpc": "2.0", "id": 4, "method": "tools/call",
                "params": {
                    "name": "ajna_evaluate_device_posture",
                    "arguments": {
                        "indicators": {
                            "platform": "android",
                            "present_paths": ["/system/bin/su"],
                            "loaded_libraries": [],
                            "build_properties": {},
                            "debugger_attached": false,
                            "selinux_enforcing": true
                        }
                    }
                }
            }),
        );
        assert_eq!(response["result"]["isError"], false);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        let output: Value = serde_json::from_str(text).unwrap();
        let report: Value =
            serde_json::from_str(output["report_json"].as_str().unwrap()).unwrap();
        assert_eq!(report["verdict"], "compromised");
        assert_eq!(output["algorithm"], "ed25519");
        assert!(!output["signature_b64"].as_str().unwrap().is_empty());
    }

    #[test]
    fn face_tool_verifies_matching_embeddings() {
        let mut state = local_state();
        let response = request(
            &mut state,
            json!({
                "jsonrpc": "2.0", "id": 5, "method": "tools/call",
                "params": {
                    "name": "ajna_verify_face",
                    "arguments": {
                        "observations": [
                            { "challenge": "blink", "confidence": 0.97, "timestamp_us": 1000 },
                            { "challenge": "turn_left", "confidence": 0.95, "timestamp_us": 2000 }
                        ],
                        "probe": [0.2, 0.4, 0.6],
                        "reference": [0.2, 0.4, 0.6]
                    }
                }
            }),
        );
        assert_eq!(response["result"]["isError"], false);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        let output: Value = serde_json::from_str(text).unwrap();
        let result: Value =
            serde_json::from_str(output["result_json"].as_str().unwrap()).unwrap();
        assert_eq!(result["verdict"], "verified");
        assert_eq!(result["liveness_passed"], true);
    }

    #[test]
    fn backend_tools_report_missing_configuration_as_tool_error() {
        let mut state = local_state();
        let response = request(
            &mut state,
            json!({
                "jsonrpc": "2.0", "id": 6, "method": "tools/call",
                "params": { "name": "ajna_verify_document", "arguments": { "payload": {} } }
            }),
        );
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("AJNA_BACKEND_URL"), "got: {text}");
    }

    #[test]
    fn unknown_tool_is_invalid_params() {
        let mut state = local_state();
        let response = request(
            &mut state,
            json!({
                "jsonrpc": "2.0", "id": 7, "method": "tools/call",
                "params": { "name": "ajna_time_travel", "arguments": {} }
            }),
        );
        assert_eq!(response["error"]["code"], INVALID_PARAMS);
    }
}
