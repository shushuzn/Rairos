//! Rairos Cortex Pro API Server
//!
//! HTTP API server for the SparksMatter multi-agent research workflow.
//!
//! # Run
//!
//! ```bash
//! cargo run --bin rairos-api --features "tools,api"
//! ```

use std::net::SocketAddr;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rairos-api")]
#[command(about = "SparksMatter Research Workflow API Server")]
struct Args {
    /// Address to listen on
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    addr: SocketAddr,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           SparksMatter API Server                       ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    rairos_cortex_pro::api::start_server(args.addr).await;
}
