use solana_program::borsh::{self, BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct AddPoolRequest {
    pub token0: String,
    pub token1: String,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct AddLiquidityRequest {
    pub amount0: u128,
    pub amount1: u128,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct SwapRequest {
    pub amount0_in: u128,
    pub amount1_in: u128,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum Instruction {
    /// Initialize AMM
    ///
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` Owner
    /// 1. `[writable]` AMM state account [b"state"]
    Init(),

    /// Add new pool
    ///
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` Owner
    /// 1. `[writable]` AMM state account [b"state"]
    /// 2. `[writable]` Pool state account [b"pool", pool_id.to_be_bytes()]
    AddPool(AddPoolRequest),

    /// Add liquidity to pool
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` User account
    /// 1. `[writable]` Pool state account [b"pool", pool_id.to_be_bytes()]
    /// 2. `[writable]` User pool shares account [b"user_pool_shares", user_key, pool_id.to_be_bytes()]
    AddLiquidity(AddLiquidityRequest),

    /// Remove liquidity from pool
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` User account
    /// 1. `[writable]` Pool state account [b"pool", pool_id.to_be_bytes()]
    /// 2. `[writable]` User pool shares account [b"user_pool_shares", user_key, pool_id.to_be_bytes()]
    RemoveLiquidity(),

    /// Swap tokens
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` User account
    /// 1. `[writable]` Pool state account [b"pool", pool_id.to_be_bytes()]
    Swap(SwapRequest),
}
