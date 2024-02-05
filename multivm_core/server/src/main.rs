use std::sync::{Arc, RwLock};

use color_eyre::{
    eyre::{eyre, Report},
    Result,
};
use explorer::{
    config::ExplorerConfig, config::GraphqlConfig as ExplorerGraphqlConfig,
    config::StorageConfig as ExplorerStorageConfige,
};
use helper::{start, NodeHelper};
use rpc::JsonRpcServer;

mod explorer;
mod helper;
mod rpc;

pub fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::{
        fmt::{self},
        prelude::*,
        EnvFilter,
    };

    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info,risc0_zkvm=warn".to_owned());
    let main_layer = fmt::layer()
        .event_format(fmt::format().with_ansi(true))
        .with_filter(EnvFilter::from(filter));
    tracing_subscriber::registry()
        .with(main_layer)
        .with(ErrorLayer::default())
        .init();
    color_eyre::install().expect("`color_eyre` must be installed fine");
}

use clap::Parser;
use tokio::task::JoinError;

/// Start MultiVM Node
#[derive(Parser, Debug)]
#[command()]
struct NodeOptions {
    #[arg(short, long)]
    db_path: Option<String>,

    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    #[arg(long, default_value_t = 8081)]
    explorer_port: u16,

    #[arg(long)]
    explorer_db_path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    install_tracing();

    let NodeOptions {
        db_path,
        port,
        explorer_port,
        explorer_db_path,
    } = NodeOptions::parse();

    let (events_tx, events_rx) = tokio::sync::broadcast::channel(32);

    let json_rpc_server = JsonRpcServer::new();
    let helper = Arc::new(RwLock::new(NodeHelper::new(db_path, events_tx)?));

    let explorer_config = ExplorerConfig {
        storage: ExplorerStorageConfige {
            sqlite_db_path: explorer_db_path,
        },
        graphql: ExplorerGraphqlConfig {
            bind_address: format!("0.0.0.0:{}", explorer_port).parse()?,
            depth_limit: 100,
            complexity_limit: 1000,
        },
    };

    let (indexer_handle, graphql_handle) = explorer::start(explorer_config, events_rx).await?;

    let node_handle = helper::start(helper.clone());

    let json_rpc_handle = json_rpc_server.start(helper, port).await?;

    let err = tokio::select! {
        r = indexer_handle => service_stopped("indexer", r),
        r = graphql_handle => service_stopped("graphql", r),
        r = node_handle => service_stopped("node", r),
        _ = json_rpc_handle.stopped() => eyre!("jsonrpc stopped"),
    };

    Err(err)
}

fn service_stopped(name: &str, result: std::result::Result<Result<()>, JoinError>) -> Report {
    match result {
        Ok(Ok(_)) => eyre!(format!("{} stopped", name)),
        Ok(Err(e)) => e,
        Err(e) => eyre!(Box::new(e)),
    }
}
