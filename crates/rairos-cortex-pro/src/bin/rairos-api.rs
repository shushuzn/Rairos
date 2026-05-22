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

#[cfg(feature = "api")]
use clap::Parser;

#[cfg(feature = "api")]
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
    #[cfg(feature = "api")]
    {
        let args = Args::parse();

        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║           SparksMatter API Server                       ║");
        println!("╚══════════════════════════════════════════════════════════════╝");

        rairos_cortex_pro::api::start_server(args.addr).await;
    }

    #[cfg(not(feature = "api"))]
    {
        eprintln!("Error: This binary requires the 'api' feature to be enabled.");
        eprintln!("Run with: cargo run --bin rairos-api --features api");
        std::process::exit(1);
    }
}
