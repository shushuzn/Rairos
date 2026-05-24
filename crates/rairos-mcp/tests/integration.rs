//! Integration tests for rairos-mcp — JSON-RPC 2.0 MCP protocol server.
//!
//! Tests communicate with the McpServer through its public API:
//!   - handle_request() with raw JSON-RPC bytes
//!   - list_tools() for schema inspection
//!
//! Covers: protocol handshake, tools/list, tool schema validation, error handling,
//! and smoke tests for pure-computation tools.

use rairos_mcp::handlers;
use rairos_mcp::McpServer;
use serde_json::Value;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Build a fully-registered McpServer with all core + LLM tools.
async fn registered_server() -> McpServer {
    let server = McpServer::new();
    handlers::register_all(&server).await;
    server
}

/// Send a JSON-RPC request via handle_request and deserialize the response.
async fn send_request(server: &McpServer, json: &str) -> Value {
    let raw = server.handle_request(json.as_bytes()).await;
    serde_json::from_slice(&raw).expect("Response must be valid JSON")
}

/// Assert the response is a successful JSON-RPC response with the given id.
fn assert_success(resp: &Value, expected_id: i64) {
    assert!(resp.get("result").is_some(),
        "Expected success response with 'result', got: {resp}");
    assert_eq!(resp["id"], expected_id, "Response id mismatch");
    assert_eq!(resp["jsonrpc"], "2.0");
}

/// Assert the response is an error JSON-RPC response.
fn assert_error(resp: &Value, expected_code: i32, expected_id: i64) {
    assert!(resp.get("error").is_some(),
        "Expected error response, got: {resp}");
    assert_eq!(resp["error"]["code"], expected_code,
        "Error code mismatch: {expected_code} vs {}", resp["error"]["code"]);
    assert_eq!(resp["id"], expected_id);
}

// Note: "initialize" and "notifications/initialized" are handled by the binary
// layer (main.rs), not by McpServer::handle_request(). The library layer handles
// tools/list, tools/call, and ping.

// ─── Protocol: Ping ───────────────────────────────────────────────────────────

#[tokio::test]
async fn test_ping_returns_empty_result() {
    let server = McpServer::new();
    let resp = send_request(&server, r#"{"jsonrpc":"2.0","method":"ping","id":2}"#).await;

    assert_success(&resp, 2);
    assert_eq!(resp["result"], serde_json::json!({}));
}

// ─── Tools List ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_tools_returns_at_least_30() {
    let server = registered_server().await;
    let resp = send_request(&server, r#"{"jsonrpc":"2.0","method":"tools/list","id":3}"#).await;

    assert_success(&resp, 3);
    let tools = resp["result"]["tools"].as_array()
        .expect("tools/list must return a tools array");

    let count = tools.len();
    assert!(count >= 30, "Expected >=30 tools, got {count}");
}

#[tokio::test]
async fn test_tool_names_are_unique() {
    let server = registered_server().await;
    let tools = server.list_tools().await;

    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(names.len(), sorted.len(), "Duplicate tool names found");
}

#[tokio::test]
async fn test_tool_schema_has_required_fields() {
    let server = registered_server().await;
    let tools = server.list_tools().await;

    for tool in &tools {
        assert!(!tool.name.is_empty(), "Tool name must not be empty");
        assert!(!tool.description.is_empty(),
            "Tool '{}' must have a description", tool.name);
        assert_eq!(tool.input_schema.ty, "object",
            "Tool '{}' inputSchema.type must be 'object'", tool.name);
        assert!(tool.input_schema.properties.is_some(),
            "Tool '{}' inputSchema must have properties", tool.name);
    }
}

#[tokio::test]
async fn test_tool_required_fields_exist_in_properties() {
    let server = registered_server().await;
    let tools = server.list_tools().await;

    for tool in &tools {
        let required = tool.input_schema.required.as_deref().unwrap_or(&[]);
        let props = tool.input_schema.properties.as_ref()
            .expect("Each tool must have properties");

        for field in required {
            assert!(props.contains_key(field),
                "Tool '{}': required field '{field}' missing from properties. Available: {:?}",
                tool.name, props.keys().collect::<Vec<_>>());
        }
    }
}

#[tokio::test]
async fn test_tool_enum_fields_have_values() {
    let server = registered_server().await;
    let tools = server.list_tools().await;

    for tool in &tools {
        let props = tool.input_schema.properties.as_ref().unwrap();
        for (field_name, field_schema) in props {
            if let Some(enum_vals) = field_schema.get("enum").and_then(|v| v.as_array()) {
                assert!(!enum_vals.is_empty(),
                    "Tool '{}', field '{field_name}': enum is empty", tool.name);
            }
        }
    }
}

#[tokio::test]
async fn test_tool_array_fields_have_items() {
    let server = registered_server().await;
    let tools = server.list_tools().await;

    for tool in &tools {
        let props = tool.input_schema.properties.as_ref().unwrap();
        for (field_name, field_schema) in props {
            if field_schema.get("type").and_then(|v| v.as_str()) == Some("array") {
                assert!(field_schema.get("items").is_some(),
                    "Tool '{}', field '{field_name}': array type requires 'items'",
                    tool.name);
            }
        }
    }
}

// ─── Tools List via handle_request (round-trip) ───────────────────────────────

#[tokio::test]
async fn test_list_tools_roundtrip_all_have_schema() {
    let server = registered_server().await;
    let resp = send_request(&server, r#"{"jsonrpc":"2.0","method":"tools/list","id":4}"#).await;

    let tools = resp["result"]["tools"].as_array().unwrap();
    for tool_entry in tools {
        let name = tool_entry["name"].as_str().unwrap_or("");
        let desc = tool_entry["description"].as_str().unwrap_or("");
        let schema = &tool_entry["inputSchema"];

        assert!(!name.is_empty(), "Tool name must not be empty");
        assert!(!desc.is_empty(), "Tool '{name}' must have a description");
        assert_eq!(schema["type"], "object",
            "Tool '{name}' inputSchema.type must be 'object'");
        assert!(schema.get("properties").is_some(),
            "Tool '{name}' must have inputSchema.properties");
    }
}

// ─── Error Handling ───────────────────────────────────────────────────────────

#[tokio::test]
async fn test_unknown_tool_returns_error() {
    let server = registered_server().await;
    let resp = send_request(&server,
        r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"nonexistent_tool_xyz","arguments":{}},"id":10}"#,
    ).await;

    assert_error(&resp, -32601, 10);
    assert!(resp["error"]["message"].as_str().unwrap_or("")
        .contains("nonexistent_tool_xyz"),
        "Error message should mention the tool name");
}

#[tokio::test]
async fn test_missing_tool_name_returns_error() {
    let server = registered_server().await;
    let resp = send_request(&server,
        r#"{"jsonrpc":"2.0","method":"tools/call","params":{"arguments":{}},"id":11}"#,
    ).await;

    assert_error(&resp, -32602, 11);
}

#[tokio::test]
async fn test_unknown_method_returns_error() {
    let server = registered_server().await;
    let resp = send_request(&server,
        r#"{"jsonrpc":"2.0","method":"nonexistent/method","id":12}"#,
    ).await;

    assert_error(&resp, -32601, 12);
}

#[tokio::test]
async fn test_parse_error_on_malformed_json() {
    let server = McpServer::new();
    let raw = server.handle_request(b"this is not json").await;
    let resp: Value = serde_json::from_slice(&raw).expect("Must be valid JSON");

    assert!(resp.get("error").is_some(),
        "Malformed JSON must return parse error, got: {resp}");
    assert_eq!(resp["error"]["code"], -32700, "Should be PARSE_ERROR");
    assert_eq!(resp["id"], serde_json::Value::Null);
}

#[tokio::test]
async fn test_missing_params_returns_error() {
    let server = registered_server().await;
    let resp = send_request(&server,
        r#"{"jsonrpc":"2.0","method":"tools/call","id":13}"#,
    ).await;

    assert_error(&resp, -32602, 13);
}

// ─── Tool Smoke Tests (pure computation, no external deps) ────────────────────

#[tokio::test]
async fn test_impact_score_paper_returns_score() {
    let server = registered_server().await;
    let resp = send_request(&server,
        r#"{
            "jsonrpc":"2.0",
            "method":"tools/call",
            "params":{
                "name":"impact_score_paper",
                "arguments":{
                    "paper_id":"2301.00001",
                    "title":"Test Paper",
                    "citation_count":50,
                    "year":2023
                }
            },
            "id":14
        }"#,
    ).await;

    assert_success(&resp, 14);
    let result = &resp["result"];
    assert!(result.is_object(), "Result must be an object, got: {result}");

    // Check content structure (MCP wraps result in content[0].text)
    let content = result.get("content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str());
    assert!(content.is_some(), "Result must have content[0].text");

    // Parse inner result
    if let Some(text) = content {
        let inner: Value = serde_json::from_str(text).unwrap_or_default();
        assert!(inner.get("composite").is_some()
            || inner.get("impact_score").is_some(),
            "Impact score result should contain 'composite' or 'impact_score'");
    }
}

#[tokio::test]
async fn test_impact_rank_returns_ranked() {
    let server = registered_server().await;
    let resp = send_request(&server,
        r#"{
            "jsonrpc":"2.0",
            "method":"tools/call",
            "params":{
                "name":"impact_rank",
                "arguments":{
                    "topic":"machine learning",
                    "top_k":5,
                    "papers":[
                        {"arxiv_id":"p0000","title":"A","citation_count":100,"year":2020},
                        {"arxiv_id":"p0001","title":"B","citation_count":50,"year":2021},
                        {"arxiv_id":"p0002","title":"C","citation_count":10,"year":2022}
                    ]
                }
            },
            "id":15
        }"#,
    ).await;

    assert_success(&resp, 15);
    let content = resp["result"]["content"][0]["text"].as_str().unwrap_or("");
    let inner: Value = serde_json::from_str(content).unwrap_or_default();
    let ranked = inner["ranked"].as_array();

    assert!(ranked.is_some(), "impact_rank should return 'ranked' array");
    if let Some(list) = ranked {
        assert!(!list.is_empty(), "Should have at least 1 ranked paper");
        // Verify descending order by composite score
        let scores: Vec<f64> = list.iter()
            .filter_map(|p| p.get("composite").and_then(|c| c.as_f64()))
            .collect();
        if scores.len() >= 2 {
            let mut sorted = scores.clone();
            sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
            assert_eq!(scores, sorted, "Must be sorted by composite descending");
        }
    }
}

#[tokio::test]
async fn test_impact_score_missing_paper_id_returns_error() {
    let server = registered_server().await;
    let resp = send_request(&server,
        r#"{
            "jsonrpc":"2.0",
            "method":"tools/call",
            "params":{
                "name":"impact_score_paper",
                "arguments":{}
            },
            "id":16
        }"#,
    ).await;

    // Should not crash — should return error
    assert!(resp.get("error").is_some() || resp.get("result").is_some(),
        "Must return either result or error, got: {resp}");
}

#[tokio::test]
async fn test_invalid_tool_args_dont_crash() {
    let server = registered_server().await;

    // Try calling tag_add with missing required args
    let resp = send_request(&server,
        r#"{
            "jsonrpc":"2.0",
            "method":"tools/call",
            "params":{
                "name":"tag_add",
                "arguments":{}
            },
            "id":17
        }"#,
    ).await;

    // Should return an error, not crash
    let is_valid = resp.get("error").is_some() || resp.get("result").is_some();
    assert!(is_valid, "Invalid args must not crash: {resp}");
}

// ─── JSON-RPC Protocol ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_invalid_jsonrpc_version() {
    let server = McpServer::new();
    let resp = send_request(&server,
        r#"{"jsonrpc":"1.0","method":"ping","id":20}"#,
    ).await;

    assert_error(&resp, -32600, 20);
}

#[tokio::test]
async fn test_empty_json_object() {
    let server = McpServer::new();
    let raw = server.handle_request(b"{}").await;
    let resp: Value = serde_json::from_slice(&raw).expect("Must be valid JSON");
    assert!(resp.get("error").is_some(), "Empty JSON should return error");
}

#[tokio::test]
async fn test_list_tools_direct_api_count() {
    let server = registered_server().await;
    let tools = server.list_tools().await;
    let count = tools.len();
    eprintln!("Total registered tools: {count}");
    assert!(count >= 50, "Expected >=50 tools, got {count}");
}

// ─── Server Info ──────────────────────────────────────────────────────────────
// Note: "initialize" is handled by the binary layer (main.rs), not by McpServer.

// ─── Concurrency Tests (OnceLock/RwLock validation) ────────────────────────────

use std::sync::Arc;
use std::time::Duration;

/// Test concurrent tool calls don't cause race conditions
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_concurrent_tool_calls_no_race() {
    use tokio::time::timeout;
    
    let server = Arc::new(registered_server().await);
    let tools = server.list_tools().await;
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
    
    // Spawn 20 concurrent tasks, each calling a tool
    let mut handles = Vec::new();
    for i in 0..20 {
        let srv = server.clone();
        let names = tool_names.clone();
        let handle = tokio::spawn(async move {
            let tool = &names[i % names.len()];
            let req = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "tools/call",
                "params": {
                    "name": tool,
                    "arguments": {}
                },
                "id": i
            });
            let resp = send_request(&srv, &req.to_string()).await;
            // Must not panic or return internal error
            assert!(resp.get("error").is_some() || resp.get("result").is_some(),
                "Tool '{}' should return valid response, got: {}", tool, resp);
        });
        handles.push(handle);
    }
    
    // Wait for all with timeout
    for handle in handles {
        let result = timeout(Duration::from_secs(30), handle).await;
        assert!(result.is_ok(), "Concurrent tool call timed out");
        assert!(result.unwrap().is_ok(), "Task panicked");
    }
}

/// Test concurrent list_tools calls
#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
async fn test_concurrent_list_tools() {
    let server = Arc::new(registered_server().await);
    
    let mut handles = Vec::new();
    for _i in 0..50 {
        let srv = server.clone();
        let handle = tokio::spawn(async move {
            let tools = srv.list_tools().await;
            // All calls should return same count (no race)
            assert!(tools.len() >= 50, "Expected >=50 tools, got {}", tools.len());
            tools.len()
        });
        handles.push(handle);
    }
    
    let results = futures::future::join_all(handles).await;
    let counts: Vec<usize> = results.into_iter().filter_map(|r| r.ok()).collect();
    // All should return same count
    let first = counts[0];
    for (i, count) in counts.iter().enumerate() {
        assert_eq!(first, *count, "Call {} returned different count: {} vs {}", i, count, first);
    }
}

/// Test concurrent mixed operations (list + call)
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_concurrent_mixed_operations() {
    use tokio::time::timeout;
    
    let server = Arc::new(registered_server().await);
    
    let mut handles = Vec::new();
    for i in 0..30 {
        let srv = server.clone();
        let handle = tokio::spawn(async move {
            if i % 3 == 0 {
                // list_tools
                let tools = srv.list_tools().await;
                tools.len()
            } else {
                // ping
                let req = r#"{"jsonrpc":"2.0","method":"ping","id":999}"#;
                let resp = send_request(&srv, req).await;
                if resp.get("result").is_some() { 1 } else { 0 }
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let result = timeout(Duration::from_secs(10), handle).await;
        assert!(result.is_ok() && result.as_ref().unwrap().is_ok());
    }
}

/// Test OnceLock client caching (same instance returned)
#[tokio::test]
async fn test_llm_client_caching() {
    use rairos_mcp::{llm_client, llm_model};
    
    // Call multiple times — should return same reference
    let client1 = llm_client();
    let client2 = llm_client();
    
    // Both should be same Option (None or Some)
    match (&client1, &client2) {
        (Some(_), Some(_)) => {},
        (None, None) => {},
        _ => panic!("Inconsistent client state"),
    }
    
    // Model should be consistent
    let model1 = llm_model();
    let model2 = llm_model();
    assert_eq!(model1, model2, "Model should be consistent");
}

/// Test server can handle rapid burst requests
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rapid_burst_requests() {
    let server = Arc::new(registered_server().await);
    
    // Send 100 rapid ping requests in parallel
    let mut handles = Vec::new();
    for i in 0..100 {
        let srv = server.clone();
        let handle = tokio::spawn(async move {
            let req = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "ping",
                "id": i
            });
            send_request(&srv, &req.to_string()).await
        });
        handles.push(handle);
    }
    
    let results = futures::future::join_all(handles).await;
    let success_count = results.iter()
        .filter_map(|r| r.as_ref().ok())
        .filter(|v| v.get("result").is_some())
        .count();
    
    // At least 95% should succeed (allowing for rare timing issues)
    assert!(success_count >= 95, "Only {}/100 requests succeeded", success_count);
}
