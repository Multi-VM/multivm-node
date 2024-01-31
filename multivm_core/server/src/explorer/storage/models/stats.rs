#[derive(Clone, Debug)]
pub struct Stats {
    pub id: i64,
    pub timestamp: i64,
    pub block_id: i64,
    pub total_txs: i64,
    pub total_accounts: i64,
    pub total_contracts: i64,
}
