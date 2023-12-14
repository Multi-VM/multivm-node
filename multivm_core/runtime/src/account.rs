use borsh::{BorshDeserialize, BorshSerialize};
use multivm_primitives::{EvmAddress, MultiVmAccountId, SolanaAddress};
use serde::Serialize;

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, Serialize)]
pub enum Executable {
    Evm(),
    MultiVm(MultiVmExecutable),
    Solana(SolanaExecutable),
}

impl From<MultiVmExecutable> for Executable {
    fn from(executable: MultiVmExecutable) -> Self {
        Self::MultiVm(executable)
    }
}

impl From<SolanaExecutable> for Executable {
    fn from(executable: SolanaExecutable) -> Self {
        Self::Solana(executable)
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, Serialize)]
pub struct MultiVmExecutable {
    pub image_id: [u32; 8],
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, Serialize)]
pub struct SolanaExecutable {
    pub image_id: [u32; 8],
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, Serialize)]
pub struct Account {
    internal_id: u128,
    pub evm_address: EvmAddress,
    pub multivm_account_id: Option<MultiVmAccountId>,
    pub solana_address: Option<SolanaAddress>,
    pub executable: Option<Executable>,
    pub balance: u128,
    pub nonce: u64,
}
