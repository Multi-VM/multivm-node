#![no_main]

use borsh::{BorshDeserialize, BorshSerialize};
use multivm_sdk::multivm_primitives::{EvmAddress, MultiVmAccountId};
use std::collections::HashMap;

pub struct AmmContract;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct State {
    pub pools: HashMap<u128, Pool>,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct Pool {
    pub id: u128,
    pub token0: Token,
    pub token1: Token,
    pub reserve0: u128,
    pub reserve1: u128,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct Token {
    pub symbol: String,
    pub address: String,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct AddLiquidity {
    pub pool_id: u128,
    pub amount0: u128,
    pub amount1: u128,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Swap {
    pub pool_id: u128,
    pub amount0_in: u128,
    pub amount1_in: u128,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct Account {
    internal_id: u128,
    pub evm_address: EvmAddress,
    pub multivm_account_id: Option<MultiVmAccountId>,
    pub public_key: Option<Vec<u8>>,
    pub executable: Option<()>,
    pub balance: u128,
    pub nonce: u64,
}


#[multivm_sdk_macros::contract]
impl AmmContract {
    pub fn init(input: String) {
        let state = State {
            pools: HashMap::new(),
        };
        Self::save(state, input);
    }

    pub fn add_pool(tokens: (String, String)) {
        let mut state = Self::load();
        
        let token0_id = AccountId::MultiVm(MultiVmAccountId::try_from(tokens.0.clone()).unwrap());
        let commitment = env::cross_contract_call(token0_id, "symbol".to_string(), 0, &());
        let symbol0: String = commitment.try_deserialize_response().unwrap();

        let token1_id = AccountId::MultiVm(MultiVmAccountId::try_from(tokens.1.clone()).unwrap());
        let commitment = env::cross_contract_call(token1_id, "symbol".to_string(), 0, &());
        let symbol1: String = commitment.try_deserialize_response().unwrap();

        let token0 = Token { symbol: symbol0, address: tokens.0 };
        let token1 = Token { symbol: symbol1, address: tokens.1 };
        let id = state.pools.len() as u128;

        let pool = Pool {
            id,
            token0,
            token1,
            reserve0: 0,
            reserve1: 0,
        };
        state.pools.insert(pool.id, pool);

        Self::save(state, id);
    }

    pub fn add_liquidity(input: AddLiquidity) {
        let mut state = Self::load();

        let caller = env::caller();
        let mut pool = state.pools.get(&input.pool_id).expect("Pool not found").clone();

        let token0_id = AccountId::MultiVm(MultiVmAccountId::try_from(pool.token0.address.clone()).unwrap());
        let _commitment = env::cross_contract_call(token0_id, "transfer_from".to_string(), 0, &(caller.clone(), input.amount0));

        let token1_id = AccountId::MultiVm(MultiVmAccountId::try_from(pool.token1.address.clone()).unwrap());
        let _commitment = env::cross_contract_call(token1_id, "transfer_from".to_string(), 0, &(caller.clone(), input.amount1));

        pool.reserve0 += input.amount0;
        pool.reserve1 += input.amount1;

        state.pools.insert(pool.id, pool);

        Self::save(state, ())
    }

    pub fn swap(input: Swap) {
        let mut state = Self::load();

        let caller = env::caller();
        let mut pool = state.pools.get(&input.pool_id).expect("Pool not found").clone();

        let amount1_out = pool.reserve1 - pool.reserve0 * pool.reserve1 / (pool.reserve0 + input.amount0_in);

        pool.reserve0 = pool.reserve0 + input.amount0_in;
        pool.reserve1 = pool.reserve1 - amount1_out;

        let token0_id = AccountId::MultiVm(MultiVmAccountId::try_from(pool.token0.address.clone()).unwrap());
        let _commitment = env::cross_contract_call(token0_id, "transfer_from".to_string(), 0, &(caller.clone(), input.amount0_in));

        let token1_id = AccountId::MultiVm(MultiVmAccountId::try_from(pool.token1.address.clone()).unwrap());
        let _commitment = env::cross_contract_call(token1_id, "transfer".to_string(), 0, &(caller.clone(), amount1_out));

        state.pools.insert(pool.id, pool);

        Self::save(state, ())
    }
}

impl AmmContract {
    fn load() -> State {
        env::get_storage("root".to_string()).expect("Contract not initialized")
    }

    fn save<T: BorshSerialize>(state: State, output: T) {
        env::set_storage("root".to_string(), state);
        env::commit(output);
    }
}

impl Pool {
    pub fn symbol(&self) -> String {
        format!("{}/{}", self.token0.symbol, self.token1.symbol).to_string()
    }
}