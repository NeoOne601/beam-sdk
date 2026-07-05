// ajna-mcp-server — Model Context Protocol server over stdio.
//
// Client businesses point their AI agent runtime (Claude, or any MCP client)
// at this binary and get Ajna verification as first-class agent tools:
//
//   { "mcpServers": { "ajna": { "command": "ajna-mcp-server" } } }
//
// Transport: MCP stdio — one JSON-RPC 2.0 message per line on stdin/stdout.
// The protocol loop is deliberately hand-rolled (~100 lines, serde_json
// only): a heavyweight SDK dependency buys nothing at this message volume.
//
// Configuration (environment):
//   AJNA_BACKEND_URL     — base URL of the Ajna verify backend; enables the
//                          ajna_verify_document / ajna_query_audit_log tools.
//   AJNA_API_KEY         — tenant API key sent as X-Api-Key to the backend.
//   AJNA_MCP_SIGN_ALGO   — "ed25519" (default) or "ml-dsa-65" for
//                          post-quantum signatures on local tool output.

mod server;
mod tools;

use std::io::{self, BufRead, Write};

fn main() {
    let mut state = server::ServerState::from_env();
    let stdin = io::stdin();
    let stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => break, // stdin closed or unreadable — clean shutdown
        };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = server::handle_message(&mut state, &line) {
            let mut out = stdout.lock();
            // A write failure means the client is gone; exit quietly.
            if writeln!(out, "{response}")
                .and_then(|()| out.flush())
                .is_err()
            {
                break;
            }
        }
    }
}
