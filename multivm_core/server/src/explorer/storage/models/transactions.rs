#[derive(Clone, Debug)]
pub struct Transaction {
    pub id: i64,
    pub hash: String,
    pub block_id: i64,
    pub signer_account_id: i64,
    pub receiver_account_id: i64,
    pub format: String,
    pub nonce: i64,
}
