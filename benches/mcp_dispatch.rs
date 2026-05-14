use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use rairos_mcp::{handlers, McpServer};

fn bench_mcp_server_init(c: &mut Criterion) {
    c.bench_function("mcp_init_register_all", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let server = McpServer::new();
                handlers::register_all(&server).await;
                black_box(server)
            });
        });
    });
}

fn bench_mcp_list_tools(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let server = rt.block_on(async {
        let s = McpServer::new();
        handlers::register_all(&s).await;
        s
    });

    c.bench_function("mcp_list_tools", |b| {
        b.iter(|| {
            rt.block_on(async {
                let tools = server.list_tools().await;
                black_box(tools)
            });
        });
    });
}

fn bench_mcp_dispatch_unknown_tool(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let server = rt.block_on(async {
        let s = McpServer::new();
        handlers::register_all(&s).await;
        s
    });

    let request = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"nonexistent_tool","arguments":{}}}"#;

    c.bench_function("mcp_dispatch_unknown_tool", |b| {
        b.iter(|| {
            let resp = rt.block_on(server.handle_request(black_box(request.as_bytes())));
            black_box(resp)
        });
    });
}

criterion_group!(
    benches,
    bench_mcp_server_init,
    bench_mcp_list_tools,
    bench_mcp_dispatch_unknown_tool,
);
criterion_main!(benches);
