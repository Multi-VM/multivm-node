use borsh::{BorshDeserialize, BorshSerialize};
use k256::ecdsa::signature::{Signer, Verifier};
use serde::{Deserialize, Serialize};

pub use k256;

pub mod syscalls;

use std::collections::HashMap;

pub const SYSTEM_META_CONTRACT_ACCOUNT_ID: &str = "multivm";

#[derive(
    Serialize,
    Deserialize,
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Clone,
    PartialEq,
    Hash,
    PartialOrd,
    Eq,
)]
pub struct ContractCallContext {
    pub contract_id: AccountId,
    pub contract_call: ContractCall,
    pub sender_id: AccountId,
    pub signer_id: AccountId,
}

impl ContractCallContext {
    pub fn try_from_bytes(bytes: Vec<u8>) -> std::io::Result<Self> {
        borsh::BorshDeserialize::deserialize(&mut bytes.as_slice())
    }

    pub fn into_bytes(&self) -> Vec<u8> {
        borsh::to_vec(&self).expect("Expected to serialize")
    }
}

#[derive(
    Serialize,
    Deserialize,
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Clone,
    PartialEq,
    Hash,
    PartialOrd,
    Eq,
    Ord,
)]
pub struct AccountId(String);

impl From<String> for AccountId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for AccountId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl ToString for AccountId {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

pub type Digest = [u8; 32];

pub type StorageKey = String;

/// Outcome of a contract call.
#[derive(
    Serialize,
    Deserialize,
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Clone,
    PartialEq,
    Hash,
    PartialOrd,
    Eq,
)]
pub struct Commitment {
    pub call_hash: Digest,
    pub response: Vec<u8>,
    pub cross_calls_hashes: Vec<(Digest, Digest)>, // hashes of cross-calls (call, commitment)
    pub previous_account_root: Option<Digest>,
    pub new_account_root: Option<Digest>,
}

impl Commitment {
    pub fn try_from_bytes(bytes: Vec<u8>) -> std::io::Result<Self> {
        borsh::BorshDeserialize::deserialize(&mut bytes.as_slice())
    }

    pub fn into_bytes(&self) -> Vec<u8> {
        borsh::to_vec(&self).expect("Expected to serialize")
    }

    pub fn try_deserialize_response<T: BorshDeserialize>(&self) -> std::io::Result<T> {
        borsh::BorshDeserialize::deserialize(&mut self.response.as_slice())
    }
}

#[derive(
    Serialize,
    Deserialize,
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Clone,
    PartialEq,
    Hash,
    PartialOrd,
    Eq,
)]
pub struct ContractCall {
    pub method: String,
    pub args: Vec<u8>,
    pub gas: u64,
    pub deposit: u128,
}

impl ContractCall {
    pub fn new<T: BorshSerialize>(method: String, args: &T, gas: u64, deposit: u128) -> Self {
        Self {
            method,
            args: borsh::to_vec(args).expect("Expected to serialize"),
            gas,
            deposit,
        }
    }

    pub fn new_raw(method: String, args: Vec<u8>, gas: u64, deposit: u128) -> Self {
        Self {
            method,
            args,
            gas,
            deposit,
        }
    }

    pub fn try_deserialize_args<T: BorshDeserialize>(&self) -> std::io::Result<T> {
        borsh::BorshDeserialize::deserialize(&mut self.args.as_slice())
    }
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    BorshSerialize,
    BorshDeserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Transaction {
    pub receiver_id: AccountId,
    pub calls: Vec<ContractCall>,
    pub signer_id: AccountId,
    pub origin_block_height: u64,
    pub origin_block_hash: Digest,
    pub deadline: u64, // in blocks
    pub nonce: u64,
}

impl Transaction {
    pub fn new(
        receiver_id: AccountId,
        calls: Vec<ContractCall>,
        signer_id: AccountId,
        origin_block_height: u64,
        origin_block_hash: Digest,
        deadline: u64,
        nonce: u64,
    ) -> Self {
        Self {
            receiver_id,
            calls,
            signer_id,
            origin_block_height,
            origin_block_hash,
            deadline,
            nonce,
        }
    }

    pub fn hash(&self) -> Digest {
        use sha2::{Digest as _, Sha256};
        let hash = Sha256::digest(borsh::to_vec(self).unwrap());
        hash.try_into().unwrap()
    }

    pub fn bytes(&self) -> Vec<u8> {
        borsh::to_vec(self).unwrap()
    }

    pub fn context(&self, call_index: usize) -> ContractCallContext {
        ContractCallContext {
            contract_id: self.receiver_id.clone(),
            contract_call: self.calls[call_index].clone(),
            sender_id: self.signer_id.clone(),
            signer_id: self.signer_id.clone(),
        }
    }
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    BorshSerialize,
    BorshDeserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Attachments {
    pub contracts_images: HashMap<[u32; 8], Vec<u8>>,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    BorshSerialize,
    BorshDeserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub signature: Vec<u8>,
    /// Unsigned attachments
    pub attachments: Option<Attachments>,
}

impl SignedTransaction {
    pub fn new(transaction: Transaction, private_key: &k256::ecdsa::SigningKey) -> Self {
        let signature: k256::ecdsa::Signature = private_key.try_sign(&transaction.bytes()).unwrap();
        Self {
            transaction,
            signature: signature.to_vec(),
            attachments: None,
        }
    }

    pub fn new_with_attachments(
        transaction: Transaction,
        private_key: &k256::ecdsa::SigningKey,
        attachments: Attachments,
    ) -> Self {
        let signature: k256::ecdsa::Signature = private_key.try_sign(&transaction.bytes()).unwrap();
        // let signature = private_key.sign(&transaction.bytes()).to_bytes().to_vec();
        Self {
            transaction,
            signature: signature.to_vec(),
            attachments: Some(attachments),
        }
    }

    pub fn verify(&self, public_key: k256::ecdsa::VerifyingKey) -> bool {
        let signature = k256::ecdsa::Signature::from_slice(&self.signature).unwrap();

        public_key
            .verify(&self.transaction.bytes(), &signature)
            .is_ok()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Block {
    pub height: u64,
    pub hash: Digest,
    pub parent_hash: Digest,
    pub previous_global_root: Digest,
    pub new_global_root: Digest,
    pub timestamp: u64,
    pub txs: Vec<SignedTransaction>,
    pub call_outputs: HashMap<Digest, Vec<u8>>,
    // pub execution_outcomes: HashMap<Digest, ExecutionOutcome>,
    // pub sessions: HashMap<Digest, String>, // TODO: replace json to struct
}

#[derive(Serialize, Deserialize, Debug, BorshSerialize, BorshDeserialize)]
pub struct TransactionBuilder {
    pub receiver_id: AccountId,
    pub calls: Vec<ContractCall>,
    pub signer_id: AccountId,
    pub origin_block_height: u64,
    pub origin_block_hash: Digest,
    pub deadline: Option<u64>,
    pub nonce: u64,
}

impl TransactionBuilder {
    pub fn new(
        receiver_id: AccountId,
        calls: Vec<ContractCall>,
        signer_id: AccountId,
        origin_block: &Block,
    ) -> Self {
        Self {
            receiver_id,
            calls,
            signer_id,
            origin_block_height: origin_block.height,
            origin_block_hash: origin_block.hash.clone(),
            deadline: None,
            nonce: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    pub fn set_nonce(mut self, nonce: u64) -> Self {
        self.nonce = nonce;
        self
    }

    pub fn set_deadline(mut self, deadline: u64) -> Self {
        self.deadline = Some(deadline);
        self
    }

    pub fn build(self) -> Transaction {
        Transaction {
            receiver_id: self.receiver_id,
            calls: self.calls,
            signer_id: self.signer_id.clone(),
            origin_block_height: self.origin_block_height,
            origin_block_hash: self.origin_block_hash.clone(),
            deadline: self.deadline.unwrap_or(10), // TODO default deadline
            nonce: self.nonce,
        }
    }
}
