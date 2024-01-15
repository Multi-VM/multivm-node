use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter};

use crate::server::MultivmServer;

mod server;
mod utils;

pub fn install_tracing() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info,risc0_zkvm=warn".to_owned());

    let main_layer = fmt::layer()
        .event_format(fmt::format().with_ansi(true))
        .with_filter(EnvFilter::from(filter));

    let registry = registry().with(main_layer);

    registry.init();
}

use clap::Parser;

/// Start MultiVM Node
#[derive(Parser, Debug)]
#[command()]
struct NodeOptions {
    #[arg(short, long)]
    db_path: Option<String>,

    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    install_tracing();

    let NodeOptions { db_path, port } = NodeOptions::parse();

    let server = MultivmServer::new();

    server.start(db_path, port).await
}
