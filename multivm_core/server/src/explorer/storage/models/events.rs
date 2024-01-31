#[derive(Clone, Debug)]
pub struct Event {
    pub id: i64,
    pub receipt_id: i64,
    pub index_in_receipt: i64,
    pub message: String,
}
