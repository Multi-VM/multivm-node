#[derive(Clone, Debug)]
pub struct Block {
    pub id: i64,
    pub number: i64,
    pub hash: String,
    pub timestamp: i64,
    pub txs_count: i64,
}
