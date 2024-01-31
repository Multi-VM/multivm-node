use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub struct StorageConfig {
    /// Path to the SQLite database file.
    /// If not set, the database will be stored in memory.
    pub sqlite_db_path: Option<String>,
}

#[derive(Clone, Debug)]
pub struct GraphqlConfig {
    /// Address to bind the GraphQL server to.
    pub bind_address: SocketAddr,
    /// Query depth limit.
    pub depth_limit: usize,
    /// Query complexity limit.
    pub complexity_limit: usize,
}

pub struct ExplorerConfig {
    pub storage: StorageConfig,
    pub graphql: GraphqlConfig,
}
