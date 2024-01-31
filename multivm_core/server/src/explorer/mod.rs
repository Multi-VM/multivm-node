pub mod config;
mod graphql;
mod indexer;
mod storage;

use multivm_primitives::Block;

use color_eyre::Result;
use tokio::task::JoinHandle;

pub async fn start(
    config: config::ExplorerConfig,
    blocks_rx: tokio::sync::broadcast::Receiver<Block>,
) -> Result<(JoinHandle<Result<()>>, JoinHandle<Result<()>>)> {
    let storage = storage::Storage::new(config.storage).await?;

    let indexer = indexer::start(storage.clone(), blocks_rx).await;

    let graphql_config = config.graphql;
    let graphql_server = graphql::start(graphql_config, storage).await?;

    Ok((indexer, graphql_server))
}
