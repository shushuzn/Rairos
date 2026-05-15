//! Rairos MCP — stdio transport for MCP protocol (JSON-RPC 2.0)
//!
//! Reads JSON-RPC requests from stdin, dispatches to McpServer, writes responses to stdout.
//! Replaces: rairos_mcp.py

use rairos_mcp::{handlers, McpServer};
use tokio::io::{AsyncBufReadExt, BufReader};

/// Handle the MCP initialize method — return server capabilities.
fn handle_initialize(id: serde_json::Value) -> Vec<u8> {
    let resp = serde_json::json!({
        "jsonrpc": "2.0",
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "rairos",
                "version": env!("CARGO_PKG_VERSION"),
            },
        },
        "id": id,
    });
    serde_json::to_vec(&resp).unwrap_or_default()
}

/// Handle a single JSON-RPC message (one line from stdin).
async fn handle_message(raw: &[u8], server: &McpServer) -> Vec<u8> {
    let msg: serde_json::Value = match serde_json::from_slice(raw) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::to_vec(&serde_json::json!({
                "jsonrpc": "2.0",
                "error": { "code": -32700, "message": format!("Parse error: {}", e) },
                "id": serde_json::Value::Null,
            }))
            .unwrap_or_default();
        }
    };

    let is_notification = msg.get("id").is_none() || msg["id"].is_null();
    let method = msg["method"].as_str().unwrap_or("").to_string();
    let id = msg.get("id").cloned().unwrap_or(serde_json::Value::Null);

    let response = match method.as_str() {
        "initialize" => handle_initialize(id),
        "notifications/initialized" | "initialized" => return Vec::new(),
        _ => server.handle_request(raw).await,
    };

    if is_notification {
        return Vec::new();
    }

    response
}

#[tokio::main]
async fn main() {
    // Build and register all tools
    let server = McpServer::new();
    handlers::register_all(&server).await;

    // Stdio transport loop
    let stdin = tokio::io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }

        let response = handle_message(line.as_bytes(), &server).await;

        if !response.is_empty() {
            if let Ok(resp_str) = String::from_utf8(response) {
                println!("{}", resp_str);
            }
        }
    }
}
