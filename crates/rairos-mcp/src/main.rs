//! MCP server binary — runs the JSON-RPC 2.0 MCP server over stdio

use rairos_mcp::{McpServer, protocol::ToolHandler};
use serde_json::Value;
use std::io::{self, Read, Write};

// Re-export tool handlers from sub-crates
mod handlers;

// Read line-delimited JSON-RPC messages from stdin, write responses to stdout
fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let rt = tokio::runtime::Runtime::new().expect("failed to create runtime");
    let server = rt.block_on(async {
        let s = McpServer::new();
        handlers::register_all(&s).await;
        s
    });

    tracing::info!("rairos-mcp server started, listening on stdin/stdout");

    loop {
        // Read one line (JSON-RPC message) at a time from stdin
        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => {
                // EOF — graceful shutdown
                tracing::info!("stdin closed, shutting down");
                break;
            }
            Ok(_) => {
                let input = line.trim_end_matches('\n').trim_end_matches('\r');
                if input.is_empty() {
                    continue;
                }
                let output = rt.block_on(server.handle_request(input.as_bytes()));
                // Write response followed by newline
                let mut out = io::stdout();
                out.write_all(&output).unwrap();
                out.write_all(b"\n").unwrap();
                out.flush().unwrap();
            }
            Err(e) => {
                eprintln!("read error: {}", e);
                break;
            }
        }
    }
}
