use borsh::{BorshDeserialize, BorshSerialize};
use k256::ecdsa::{signature::Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

pub use k256;

pub mod syscalls;

use std::{collections::HashMap, str::FromStr};

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
    Solana(SolanaAddress),
}

impl AccountId {
    /// Returns the system metacontract account id
    pub fn system_meta_contract() -> Self {
        Self::MultiVm(MultiVmAccountId::try_from("multivm".to_string()).unwrap())
    }

    pub fn multivm(&self) -> MultiVmAccountId {
        match self {
            AccountId::MultiVm(account_id) => account_id.clone(),
            _ => panic!("Not a multiVM account"),
        }
    }

    pub fn evm(&self) -> EvmAddress {
        match self {
            AccountId::Evm(address) => address.clone(),
            _ => panic!("Not a EVM account"),
        }
    }

    pub fn solana(&self) -> SolanaAddress {
        match self {
            AccountId::Solana(address) => address.clone(),
            _ => panic!("Not a Solana account"),
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

impl From<SolanaAddress> for AccountId {
    fn from(id: SolanaAddress) -> Self {
        Self::Solana(id)
    }
}

impl std::fmt::Display for AccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountId::MultiVm(id) => write!(f, "{}", id),
            AccountId::Evm(id) => write!(f, "{}", id),
            AccountId::Solana(id) => write!(f, "{}", id),
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

impl EvmAddress {
    pub fn to_bytes(&self) -> [u8; 20] {
        self.0
    }
}

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

impl FromStr for EvmAddress {
    type Err = AddressParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let bytes = ethers_core::utils::hex::decode(value.replace("0x", ""))?;
        let arr: [u8; 20] = bytes
            .try_into()
            .map_err(|_| AddressParseError::InvalidLength)?;

        Ok(arr.into())
    }
}

impl From<EvmAddress> for eth_primitive_types::H160 {
    fn from(id: EvmAddress) -> Self {
        id.0.into()
    }
}

impl From<VerifyingKey> for EvmAddress {
    fn from(value: VerifyingKey) -> Self {
        let point = value.to_encoded_point(false);
        let hash = ethers_core::utils::keccak256(&point.as_bytes()[1..]);
        eth_primitive_types::H160::from_slice(&hash[12..]).into()
    }
}

impl std::fmt::Display for EvmAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = self.0.iter().map(|byte| format!("{:02x}", byte)).collect();
        write!(f, "{}", s)
    }
}

impl std::fmt::Debug for EvmAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = self.0.iter().map(|byte| format!("{:02x}", byte)).collect();
        write!(f, "{}", s)
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
pub struct SolanaAddress([u8; 32]);

impl SolanaAddress {
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

impl From<[u8; 32]> for SolanaAddress {
    fn from(id: [u8; 32]) -> Self {
        Self(id)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AddressParseError {
    InvalidLength,
    Base58(bs58::decode::Error),
    Hex(ethers_core::utils::hex::FromHexError),
}

impl std::error::Error for AddressParseError {}

impl std::fmt::Display for AddressParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Self::InvalidLength => write!(f, "invalid length"),
            Self::Base58(e) => write!(f, "base58 error: {}", e),
            Self::Hex(e) => write!(f, "hex error: {}", e),
        }
    }
}

impl From<bs58::decode::Error> for AddressParseError {
    fn from(e: bs58::decode::Error) -> Self {
        Self::Base58(e)
    }
}

impl From<ethers_core::utils::hex::FromHexError> for AddressParseError {
    fn from(e: ethers_core::utils::hex::FromHexError) -> Self {
        Self::Hex(e)
    }
}

impl FromStr for SolanaAddress {
    type Err = AddressParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let bytes = bs58::decode(value).into_vec()?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| AddressParseError::InvalidLength)?;

        Ok(arr.into())
    }
}

impl std::fmt::Display for SolanaAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = bs58::encode(&self.0).into_string();
        write!(f, "{}", s)
    }
}

pub type Digest = [u8; 32];

pub type StorageKey = String;

pub type ContractResponse = Result<Vec<u8>, ContractError>;

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
pub enum Event {
    System(SystemEvent),
    Contract(Vec<u8>),
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
pub enum SystemEvent {
    AccountCreated(EvmAddress, Option<MultiVmAccountId>, SolanaAddress),
    ContractDeployed(AccountId),
    BalanceChanged(AccountId, u128),
}

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
    pub response: ContractResponse,
    pub events: Vec<Event>,
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
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Receipt {
    pub contract_id: AccountId,
    pub call: ContractCall,
    pub response: ContractResponse,
    pub gas_used: u64,
    pub events: Vec<Event>,
    pub cross_calls_receipts: Vec<Receipt>,
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
pub struct ContractError {
    message: String,
}

impl ContractError {
    pub fn new(message: String) -> Self {
        Self { message }
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

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct EthereumTransactionRequest(Vec<u8>);

impl EthereumTransactionRequest {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn decode(
        &self,
    ) -> (
        ethers_core::types::TransactionRequest,
        ethers_core::types::Signature,
    ) {
        let rlp = ethers_core::utils::rlp::Rlp::new(&self.0);
        ethers_core::types::TransactionRequest::decode_signed_rlp(&rlp).unwrap()
    }
}

// TODO: rename
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum SupportedTransaction {
    MultiVm(SignedTransaction),
    Evm(EthereumTransactionRequest),
    Solana(Vec<u8>),
}

impl SupportedTransaction {
    pub fn hash(&self) -> Digest {
        match self {
            Self::MultiVm(tx) => tx.transaction.hash(),
            Self::Evm(tx) => ethers_core::utils::keccak256::<Vec<u8>>(tx.0.clone().into()),
            Self::Solana(tx) => ethers_core::utils::keccak256::<Vec<u8>>(tx.clone().into()),
        }
    }

    pub fn to_system(&self) -> bool {
        match self {
            Self::MultiVm(tx) => tx.transaction.receiver_id == AccountId::system_meta_contract(),
            Self::Evm(_) => true,
            Self::Solana(_) => false,
        }
    }

    pub fn signer(&self) -> AccountId {
        match self {
            Self::MultiVm(tx) => tx.transaction.signer_id.clone(),
            Self::Evm(tx) => {
                let (tx_request, _sig) = tx.decode();
                let from = tx_request.from.unwrap();
                AccountId::Evm(from.into())
            }
            Self::Solana(_) => todo!(),
        }
    }

    pub fn receiver(&self) -> AccountId {
        match self {
            Self::MultiVm(tx) => tx.transaction.receiver_id.clone(),
            Self::Evm(tx) => {
                let (tx_request, _sig) = tx.decode();
                let to = tx_request
                    .to
                    .map(|to| *to.as_address().unwrap())
                    .unwrap_or(Default::default());
                AccountId::Evm(to.into())
            }
            Self::Solana(_) => todo!(),
        }
    }

    pub fn nonce(&self) -> u64 {
        match self {
            Self::MultiVm(tx) => tx.transaction.nonce,
            Self::Evm(tx) => {
                let (tx_request, _sig) = tx.decode();
                tx_request.nonce.unwrap_or_default().as_u64()
            }
            Self::Solana(_) => todo!(),
        }
    }

    pub fn deposit(&self) -> u128 {
        match self {
            Self::MultiVm(tx) => tx.transaction.calls.iter().map(|call| call.deposit).sum(),
            Self::Evm(tx) => {
                let (tx_request, _sig) = tx.decode();
                tx_request.value.unwrap_or_default().as_u128()
            }
            Self::Solana(_) => todo!(),
        }
    }
}

impl From<SignedTransaction> for SupportedTransaction {
    fn from(tx: SignedTransaction) -> Self {
        Self::MultiVm(tx)
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

    pub fn verify(&self, address: EvmAddress) -> bool {
        let Some(public_key) = self.recover() else {
            return false;
        };

        let signature = k256::ecdsa::Signature::from_slice(&self.signature).unwrap();

        let signature_valid = public_key
            .verify(&self.transaction.bytes(), &signature)
            .is_ok();

        let recovered_address = EvmAddress::from(public_key);
        let address_valid = recovered_address == address;

        if !signature_valid {
            panic!("signature is invalid");
        }

        if !address_valid {
            panic!("address is invalid: {} != {}", recovered_address, address)
        }

        signature_valid && address_valid
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
    pub call_outputs: HashMap<Digest, ContractResponse>,
    pub receipts: HashMap<Digest, Receipt>,
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
