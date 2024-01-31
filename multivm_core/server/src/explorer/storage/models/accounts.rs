#[derive(Clone, Debug)]
pub struct Account {
    pub id: i64,
    pub fvm_address: Option<String>,
    pub evm_address: String,
    pub svm_address: String,
    pub created_at_block_id: i64,
    pub modified_at_block_id: i64,
    pub executable_type: Option<String>,
    pub native_balance: String,
}
