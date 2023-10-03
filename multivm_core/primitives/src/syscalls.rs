use borsh::{BorshDeserialize, BorshSerialize};

use crate::{AccountId, ContractCall, Digest, StorageKey};

risc0_zkvm_platform::declare_syscall!(pub CROSS_CONTRACT_CALL);

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CrossContractCallRequest {
    pub contract_id: AccountId,
    pub contract_call: ContractCall,
}

impl CrossContractCallRequest {
    pub fn new(contract_id: AccountId, contract_call: ContractCall) -> Self {
        Self {
            contract_id,
            contract_call,
        }
    }
}

risc0_zkvm_platform::declare_syscall!(pub GET_STORAGE_CALL);

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct GetStorageRequest {
    pub key: StorageKey,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct GetStorageResponse {
    pub storage: Option<Vec<u8>>,
}

risc0_zkvm_platform::declare_syscall!(pub SET_STORAGE_CALL);

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct SetStorageRequest {
    pub key: StorageKey,
    pub hash: Digest,
    pub storage: Vec<u8>,
}

risc0_zkvm_platform::declare_syscall!(pub DEPLOY_CONTRACT_CALL);

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct DeployContractRequest {
    pub account_id: AccountId,
    pub image_id: [u32; 8],
}
