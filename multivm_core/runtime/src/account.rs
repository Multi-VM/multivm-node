use borsh::{BorshDeserialize, BorshSerialize};
use multivm_primitives::{EvmAddress, MultiVmAccountId};
use serde::Serialize;

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, Serialize)]
pub enum Executable {
    Evm(),
    MultiVm(MultiVmExecutable),
}

impl From<MultiVmExecutable> for Executable {
    fn from(executable: MultiVmExecutable) -> Self {
        Self::MultiVm(executable)
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, Serialize)]
pub struct MultiVmExecutable {
    pub image_id: [u32; 8],
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, Serialize)]
pub struct Account {
    internal_id: u128,
    pub evm_address: EvmAddress,
    pub multivm_account_id: Option<MultiVmAccountId>,
    pub executable: Option<Executable>,
    pub balance: u128,
    pub nonce: u64,
}
