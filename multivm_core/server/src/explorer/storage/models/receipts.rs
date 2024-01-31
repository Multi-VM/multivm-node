#[derive(Clone, Debug)]
pub struct Receipt {
    pub id: i64,
    pub transaction_id: i64,
    pub parent_receipt_id: Option<i64>,
    pub index_in_transaction: i64,
    pub result: bool,
    pub response: Option<String>,
    pub gas_used: i64,
    pub contract_account_id: i64,
    pub call_method: String,
    pub call_args: String,
    pub call_gas: i64,
    pub call_deposit: String,
}
