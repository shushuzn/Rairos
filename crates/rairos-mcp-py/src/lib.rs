use pyo3::prelude::*;
use rairos_mcp::McpServer;
use std::sync::OnceLock;

fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| tokio::runtime::Runtime::new().expect("failed to create tokio runtime"))
}

fn server() -> &'static McpServer {
    static S: OnceLock<McpServer> = OnceLock::new();
    S.get_or_init(|| {
        let s = McpServer::new();
        runtime().block_on(rairos_mcp::handlers::register_all(&s));
        s
    })
}

/// Call a tool via the Rust MCP server.
/// Returns the result text on success, None if tool not found.
#[pyfunction]
fn call_tool_rs(name: &str, arguments_json: &str) -> PyResult<Option<String>> {
    let server = server();
    let rt = runtime();

    let arguments: serde_json::Value = serde_json::from_str(arguments_json)
        .unwrap_or(serde_json::Value::Null);

    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": arguments,
        },
    });

    let request_bytes = serde_json::to_vec(&request_body)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    let response_bytes = rt.block_on(async { server.handle_request(&request_bytes).await });

    let response: serde_json::Value = serde_json::from_slice(&response_bytes)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    if let Some(error) = response.get("error") {
        let code = error["code"].as_i64().unwrap_or(0);
        if code == -32601 {
            return Ok(None);
        }
        return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("MCP error: {}", error["message"].as_str().unwrap_or("unknown")),
        ));
    }

    if let Some(result) = response.get("result") {
        let text = result["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["text"].as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| result.to_string());
        Ok(Some(text))
    } else {
        Ok(None)
    }
}

/// List all tool names available in the Rust MCP server.
#[pyfunction]
fn list_tools_rs() -> PyResult<Vec<String>> {
    let server = server();

    let request_bytes = serde_json::to_vec(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
    })).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    let response_bytes = runtime().block_on(async { server.handle_request(&request_bytes).await });
    let response: serde_json::Value = serde_json::from_slice(&response_bytes)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    let tools = response["result"]["tools"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t["name"].as_str().map(|n| n.to_string()))
                .collect()
        })
        .unwrap_or_default();
    Ok(tools)
}

/// List all tool names with full definitions (name, description, inputSchema).
#[pyfunction]
fn list_tools_detailed_rs() -> PyResult<String> {
    let server = server();

    let request_bytes = serde_json::to_vec(&serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/list",
    })).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    let response_bytes = runtime().block_on(async { server.handle_request(&request_bytes).await });
    let response: serde_json::Value = serde_json::from_slice(&response_bytes)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    let tools_json = response["result"]["tools"].to_string();
    Ok(tools_json)
}

#[pymodule]
fn rairos_mcp_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(call_tool_rs, m)?)?;
    m.add_function(wrap_pyfunction!(list_tools_rs, m)?)?;
    m.add_function(wrap_pyfunction!(list_tools_detailed_rs, m)?)?;
    Ok(())
}
