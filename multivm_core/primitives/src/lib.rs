use borsh::{BorshDeserialize, BorshSerialize};
use k256::ecdsa::signature::Verifier;
use risc0_zkvm::sha::{Impl as HashImpl, Sha256};
use serde::{Deserialize, Serialize};

pub use k256;

pub mod syscalls;

use std::collections::HashMap;

pub const CHAIN_ID: u64 = 1044942;

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
    pub environment: EnvironmentContext,
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
)]
pub struct EnvironmentContext {
    pub block_height: u64,
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
pub enum AccountId {
    MultiVm(MultiVmAccountId),
    Evm(EvmAddress),
}

impl AccountId {
    /// Returns the system metacontract account id
    pub fn system_meta_contract() -> Self {
        Self::MultiVm(MultiVmAccountId::try_from("multivm".to_string()).unwrap())
    }

    pub fn multivm(&self) -> MultiVmAccountId {
        match self {
            AccountId::MultiVm(account_id) => account_id.clone(),
            AccountId::Evm(_) => panic!("Not a multiVM account"),
        }
    }

    pub fn evm(&self) -> EvmAddress {
        match self {
            AccountId::MultiVm(_) => panic!("Not a EVM account"),
            AccountId::Evm(address) => address.clone(),
        }
    }
}

impl From<MultiVmAccountId> for AccountId {
    fn from(id: MultiVmAccountId) -> Self {
        Self::MultiVm(id)
    }
}

impl From<EvmAddress> for AccountId {
    fn from(id: EvmAddress) -> Self {
        Self::Evm(id)
    }
}

impl std::fmt::Display for AccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountId::MultiVm(id) => write!(f, "{}", id),
            AccountId::Evm(id) => write!(f, "{}", id),
        }
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
pub struct MultiVmAccountId(String);

impl MultiVmAccountId {
    const MAX_LENGTH: usize = 64;
    const ALLWED_SYMBOLS: &'static str = "abcdefghijklmnopqrstuvwxyz0123456789_-.";

    fn validate_account_id(id: &str) -> Result<(), std::io::Error> {
        if id.len() > MultiVmAccountId::MAX_LENGTH {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "AccountID is too long",
            ));
        }

        if id.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "AccountID is empty",
            ));
        }

        if id
            .to_lowercase()
            .chars()
            .any(|c| !MultiVmAccountId::ALLWED_SYMBOLS.contains(c))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "AccountID contains invalid symbols",
            ));
        }

        Ok(())
    }
}

impl TryFrom<String> for MultiVmAccountId {
    type Error = std::io::Error;

    fn try_from(id: String) -> Result<Self, Self::Error> {
        Self::validate_account_id(id.as_str()).map(|_| Self(id))
    }
}

impl TryFrom<&str> for MultiVmAccountId {
    type Error = std::io::Error;

    fn try_from(id: &str) -> Result<Self, Self::Error> {
        Self::validate_account_id(id).map(|_| Self(id.to_string()))
    }
}

impl std::fmt::Display for MultiVmAccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
pub struct EvmAddress([u8; 20]);

impl From<[u8; 20]> for EvmAddress {
    fn from(id: [u8; 20]) -> Self {
        Self(id)
    }
}

impl From<eth_primitive_types::H160> for EvmAddress {
    fn from(id: eth_primitive_types::H160) -> Self {
        Self(id.into())
    }
}

impl From<EvmAddress> for eth_primitive_types::H160 {
    fn from(id: EvmAddress) -> Self {
        id.0.into()
    }
}

impl std::fmt::Display for EvmAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = self.0.iter().map(|byte| format!("{:02x}", byte)).collect();
        write!(f, "{}", s)
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

    pub fn new_call<T: BorshSerialize>(method: &str, args: &T) -> Self {
        Self {
            method: method.into(),
            args: borsh::to_vec(args).expect("Expected to serialize"),
            gas: 300_000,
            deposit: 0,
        }
    }

    pub fn try_deserialize_args<T: BorshDeserialize>(&self) -> std::io::Result<T> {
        borsh::BorshDeserialize::deserialize(&mut self.args.as_slice())
    }
}

// TODO: rename
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum SupportedTransaction {
    MultiVm(SignedTransaction),
    Evm(Vec<u8>),
}

impl SupportedTransaction {
    pub fn hash(&self) -> Digest {
        match self {
            Self::MultiVm(tx) => tx.transaction.hash(),
            // TODO: replace with proper hash
            Self::Evm(tx) => HashImpl::hash_bytes(&tx).as_bytes().try_into().unwrap(),
        }
    }

    pub fn to_system(&self) -> bool {
        match self {
            Self::MultiVm(tx) => tx.transaction.receiver_id == AccountId::system_meta_contract(),
            SupportedTransaction::Evm(_) => true,
        }
    }
}

impl From<SignedTransaction> for SupportedTransaction {
    fn from(tx: SignedTransaction) -> Self {
        Self::MultiVm(tx)
    }
}

impl From<Vec<u8>> for SupportedTransaction {
    fn from(tx: Vec<u8>) -> Self {
        Self::Evm(tx)
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

    pub fn context(
        &self,
        call_index: usize,
        environment: EnvironmentContext,
    ) -> ContractCallContext {
        ContractCallContext {
            contract_id: self.receiver_id.clone(),
            contract_call: self.calls[call_index].clone(),
            sender_id: self.signer_id.clone(),
            signer_id: self.signer_id.clone(),
            environment,
        }
    }

    pub fn all_contexts(&self, environment: EnvironmentContext) -> Vec<ContractCallContext> {
        self.calls
            .iter()
            .map(|call| ContractCallContext {
                contract_id: self.receiver_id.clone(),
                contract_call: call.clone(),
                sender_id: self.signer_id.clone(),
                signer_id: self.signer_id.clone(),
                environment: environment.clone(),
            })
            .collect()
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
    pub recovery_id: u8,
    /// Unsigned attachments
    pub attachments: Option<Attachments>,
}

impl SignedTransaction {
    pub fn new(transaction: Transaction, private_key: &k256::ecdsa::SigningKey) -> Self {
        let (signature, recovery_id) = private_key.sign_recoverable(&transaction.bytes()).unwrap();

        Self {
            transaction,
            signature: signature.to_vec(),
            recovery_id: recovery_id.to_byte(),
            attachments: None,
        }
    }

    pub fn new_with_attachments(
        transaction: Transaction,
        private_key: &k256::ecdsa::SigningKey,
        attachments: Attachments,
    ) -> Self {
        let (signature, recovery_id) = private_key.sign_recoverable(&transaction.bytes()).unwrap();
        Self {
            transaction,
            signature: signature.to_vec(),
            recovery_id: recovery_id.to_byte(),
            attachments: Some(attachments),
        }
    }

    pub fn verify(&self, public_key: k256::ecdsa::VerifyingKey) -> bool {
        let signature = k256::ecdsa::Signature::from_slice(&self.signature).unwrap();

        public_key
            .verify(&self.transaction.bytes(), &signature)
            .is_ok()
    }

    pub fn recover(&self) -> Option<k256::ecdsa::VerifyingKey> {
        let signature = k256::ecdsa::Signature::from_slice(&self.signature).unwrap();
        let bytes = &self.transaction.bytes();
        k256::ecdsa::VerifyingKey::recover_from_msg(
            &bytes,
            &signature,
            self.recovery_id.try_into().unwrap(),
        )
        .ok()
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
    pub txs: Vec<SupportedTransaction>,
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
