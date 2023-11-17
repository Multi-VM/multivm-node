use eth_primitive_types::{H160, H256, U256};
use ethers_core::types::{Signature, TransactionRequest};
use multivm_primitives::{Block, Digest, SupportedTransaction};
use serde::{Deserialize, Serialize};

pub trait To0x {
    fn to_0x(&self) -> String;
}

impl To0x for String {
    fn to_0x(&self) -> String {
        let prefix = if self.starts_with("0x") { "" } else { "0x" };
        let body = if self.is_empty() { "0" } else { self };
        format!("{}{}", prefix, body).to_string()
    }
}

impl To0x for Vec<u8> {
    fn to_0x(&self) -> String {
        hex::encode(self).to_0x()
    }
}

impl To0x for u64 {
    fn to_0x(&self) -> String {
        format!("0x{:x?}", self).to_string()
    }
}

impl To0x for u128 {
    fn to_0x(&self) -> String {
        format!("0x{:x?}", self).to_string()
    }
}

impl To0x for usize {
    fn to_0x(&self) -> String {
        format!("0x{:x?}", self).to_string()
    }
}

impl To0x for Digest {
    fn to_0x(&self) -> String {
        hex::encode(self).to_0x()
    }
}

impl To0x for H160 {
    fn to_0x(&self) -> String {
        format!("{:#x}", self).to_string()
    }
}

impl To0x for H256 {
    fn to_0x(&self) -> String {
        format!("{:#x}", self).to_string()
    }
}

impl To0x for U256 {
    fn to_0x(&self) -> String {
        format!("{:#032X}", self).to_string()
    }
}

pub trait From0x<T> {
    fn from_0x(&self) -> T;
}

impl From0x<u64> for String {
    fn from_0x(&self) -> u64 {
        u64::from_str_radix(&self.replace("0x", ""), 16).unwrap()
    }
}

impl From0x<H160> for String {
    fn from_0x(&self) -> H160 {
        H160::from_slice(&hex::decode(self.replace("0x", "")).unwrap())
    }
}

impl From0x<String> for String {
    fn from_0x(&self) -> String {
        self.replace("0x", "")
    }
}

impl From0x<Vec<u8>> for String {
    fn from_0x(&self) -> Vec<u8> {
        hex::decode(self).unwrap()
    }
}

pub trait EthDefaults {
    fn default_hash() -> String;
    fn default_address() -> String;
    fn default_zero() -> String;
}

impl EthDefaults for String {
    fn default_hash() -> String {
        "0x0000000000000000000000000000000000000000000000000000000000000000".to_string()
    }

    fn default_address() -> String {
        "0x0000000000000000000000000000000000000000".to_string()
    }

    fn default_zero() -> String {
        "0x0".to_string()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthTransactionInput {
    pub from: String,
    pub to: String,
    pub gas: String,
    pub gas_price: String,
    pub value: String,
    pub data: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EthBlockOutput {
    pub difficulty: String,
    pub extra_data: String,
    pub gas_limit: String,
    pub gas_used: String,
    pub hash: String,
    pub logs_bloom: String,
    pub miner: String,
    pub mix_hash: String,
    pub nonce: String,
    pub number: String,
    pub parent_hash: String,
    pub receipts_root: String,
    pub sha3_uncles: String,
    pub size: String,
    pub state_root: String,
    pub timestamp: String,
    pub total_difficulty: String,
    pub transactions: Vec<String>,
    pub transactions_root: String,
    // pub uncles: String,
}

impl EthBlockOutput {
    pub fn from(block: &Block) -> Self {
        Self {
            difficulty: String::default_zero(),
            extra_data: String::default_hash(),
            gas_limit: String::default_zero(),
            gas_used: String::default_zero(),
            hash: block.hash.to_0x(),
            logs_bloom: format!("0x{}", (0..512).map(|_| "0").collect::<String>()),
            miner: String::default_address(),
            mix_hash: String::default_hash(),
            nonce: "0x0000000000000000".to_string(),
            number: block.height.to_0x(),
            parent_hash: block.parent_hash.to_0x(),
            receipts_root: block.previous_global_root.to_0x(),
            sha3_uncles: String::default_hash(),
            size: String::default_zero(),
            state_root: block.new_global_root.to_0x(),
            timestamp: block.timestamp.to_0x(),
            total_difficulty: String::default_zero(),
            transactions: block.txs.iter().map(|tx| tx.hash().to_0x()).collect(),
            transactions_root: String::default_hash(),
            // uncles: []
        }
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EthTransaction {
    pub block_hash: String,
    pub block_number: String,
    pub from: Option<String>,
    pub gas: String,
    pub gas_price: String,
    pub hash: String,
    pub input: Option<String>,
    pub nonce: String,
    pub to: Option<String>,
    pub transaction_index: String,
    pub value: String,
    pub v: String,
    pub r: String,
    pub s: String,
}

impl EthTransaction {
    pub fn from(tx: TransactionRequest, sig: Signature, hash: String, block: Block) -> Self {
        Self {
            block_hash: block.hash.to_0x(),
            block_number: block.height.to_0x(),
            from: tx.from.map(|x| x.to_0x()),
            gas: tx.gas.unwrap().to_string().to_0x(),
            gas_price: tx.gas_price.unwrap().to_string().to_0x(),
            hash,
            // input: tx.data.map(|x| x.to_vec().to_0x()),
            input: Some("0x68656c6c6f21".to_string()),
            nonce: tx.nonce.unwrap().to_string(),
            to: tx.to.map(|x| x.as_address().unwrap().to_0x()),
            transaction_index: "0x1".to_string(),
            value: tx.value.unwrap().to_0x(),
            v: sig.v.to_0x(),
            r: sig.r.to_0x(),
            s: sig.s.to_0x(),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EthTransactionReceipt {
    pub block_hash: String,
    pub block_number: String,
    pub contract_address: Option<String>, // string of the address if it was created
    pub cumulative_gas_used: String,
    pub effective_gas_price: String,
    pub from: String,
    pub gas_used: String,
    pub logs: Vec<String>,
    pub logs_bloom: String,
    pub status: String,
    pub to: String,
    pub transaction_hash: String,
    pub transaction_index: String,
    #[serde(rename = "type")]
    pub tx_type: String,
}

impl EthTransactionReceipt {
    pub fn from(tx: &SupportedTransaction, hash: String, block: &Block) -> Self {
        let from = "0x69288587af88e2c6f43832F482334a01F30e2F01";
        let to = "0x06A85356DCb5b307096726FB86A78c59D38e08ee";
        Self {
            block_hash: block.hash.to_0x(),
            block_number: block.height.to_0x(),
            contract_address: None,
            cumulative_gas_used: "0x0".to_string(),
            effective_gas_price: "0x0".to_string(),
            from: from.to_string(),
            gas_used: "0x0".to_string(),
            logs: vec![],
            logs_bloom: format!("0x{}", (0..128).map(|_| "0").collect::<String>()),
            status: "0x1".to_string(),
            to: to.to_string(),
            transaction_hash: hash.to_0x(),
            transaction_index: "0x0".to_string(),
            tx_type: "0x2".to_string(),
        }
    }
}
