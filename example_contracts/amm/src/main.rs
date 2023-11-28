#![no_main]

use borsh::{BorshDeserialize, BorshSerialize};
use eth_primitive_types::H160;
use multivm_sdk::multivm_primitives::{EvmAddress, MultiVmAccountId};
use num::integer::Roots;
use std::{collections::HashMap, str::FromStr, cmp::min};

pub struct AmmContract;

#[derive(BorshSerialize, BorshDeserialize, Default)]
pub struct State {
    pub pools: HashMap<u128, Pool>,
    pub shares: HashMap<AccountId, HashMap<u128, u128>>,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct Pool {
    pub id: u128,
    pub token0: Token,
    pub token1: Token,
    pub reserve0: u128,
    pub reserve1: u128,
    pub total_shares: u128,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct Token {
    pub symbol: String,
    pub address: String,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct AddPool {
    pub token0: String,
    pub token1: String,
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
    pub fn init() {
        let state = State {
            pools: HashMap::new(),
            shares: HashMap::new(),

        };
        Self::save(state, ());
    }

    pub fn get_pools() {
        let state = Self::load();
        let pools = state.pools.iter().map(|(_, pool)| pool.clone()).collect::<Vec<_>>();
        env::commit(pools);
    }

    pub fn get_shares(account_id: AccountId) {
        let state = Self::load();
        let default = HashMap::new();
        let shares = state.shares.get(&account_id).unwrap_or(&default);
        env::commit(shares);
    }

    pub fn get_pool(id: u128) {
        let state = Self::load();
        let pool = state.pools.iter().find_map(|(pool_id, pool)| {
            if id == *pool_id {
                Some(pool.clone())
            } else {
                None
            }
        });
        env::commit(pool);
    }

    pub fn add_pool(input: AddPool) {
        let mut state = Self::load();

        // let token0_id = AccountId::Evm(
        //     EvmAddress::try_from(H160::from_str(input.token0.clone().as_str()).unwrap()).unwrap(),
        // );
        let token0_id = AccountId::MultiVm(MultiVmAccountId::try_from(input.token0.clone()).unwrap());
        let commitment = env::cross_contract_call(token0_id, "symbol".to_string(), 0, &());
        let symbol0: String = commitment.try_deserialize_response().unwrap();

        // let token1_id = AccountId::Evm(
        //     EvmAddress::try_from(H160::from_str(input.token1.clone().as_str()).unwrap()).unwrap(),
        // );
        let token1_id = AccountId::MultiVm(MultiVmAccountId::try_from(input.token1.clone()).unwrap());
        let commitment = env::cross_contract_call(token1_id, "symbol".to_string(), 0, &());
        let symbol1: String = commitment.try_deserialize_response().unwrap();

        let token0 = Token {
            symbol: symbol0,
            address: input.token0,
        };
        let token1 = Token {
            symbol: symbol1,
            address: input.token1,
        };
        let id = state.pools.len() as u128;

        let pool = Pool {
            id,
            token0,
            token1,
            reserve0: 0,
            reserve1: 0,
            total_shares: 0,
        };
        state.pools.insert(pool.id, pool);

        Self::save(state, id);
    }

    pub fn add_liquidity(input: AddLiquidity) {
        let mut state = Self::load();

        let caller = env::caller();
        let mut pool = state
            .pools
            .get(&input.pool_id)
            .expect("Pool not found")
            .clone();

        let token0_id =
            AccountId::MultiVm(MultiVmAccountId::try_from(pool.token0.address.clone()).unwrap());
        let _commitment = env::cross_contract_call(
            token0_id,
            "transfer_from".to_string(),
            0,
            &(caller.clone(), input.amount0),
        );

        let token1_id =
            AccountId::MultiVm(MultiVmAccountId::try_from(pool.token1.address.clone()).unwrap());
        let _commitment = env::cross_contract_call(
            token1_id,
            "transfer_from".to_string(),
            0,
            &(caller.clone(), input.amount1),
        );

        let shares = if pool.total_shares == 0 {
            (input.amount0 * input.amount1).sqrt()
        } else {
            min(
                (input.amount0 * pool.total_shares) / pool.reserve0,
                (input.amount1 * pool.total_shares) / pool.reserve1,
            )
        };

        pool.total_shares += shares;
        pool.reserve0 += input.amount0;
        pool.reserve1 += input.amount1;

        let user_shares = state.shares.entry(caller).or_insert(HashMap::new());
        let user_pool_shares = user_shares.entry(pool.id).or_insert(0);
        *user_pool_shares += shares;

        state.pools.insert(pool.id, pool);

        Self::save(state, shares);
    }

    pub fn remove_liquidity(pool_id: u128) {
        let mut state = Self::load();

        let caller = env::caller();
        let mut pool = state
            .pools
            .get(&pool_id)
            .expect("Pool not found")
            .clone();
        let user_shares = state.shares.entry(caller.clone()).or_insert(HashMap::new());
        let user_pool_shares = user_shares.entry(pool.id).or_insert(0);
        let shares = *user_pool_shares;
        
        let amount0 = shares * pool.reserve0 / pool.total_shares;
        let amount1 = shares * pool.reserve1 / pool.total_shares;

        let token0_id =
            AccountId::MultiVm(MultiVmAccountId::try_from(pool.token0.address.clone()).unwrap());
        let _commitment = env::cross_contract_call(
            token0_id,
            "transfer".to_string(),
            0,
            &(caller.clone(), amount0),
        );

        let token1_id =
            AccountId::MultiVm(MultiVmAccountId::try_from(pool.token1.address.clone()).unwrap());
        let _commitment = env::cross_contract_call(
            token1_id,
            "transfer".to_string(),
            0,
            &(caller.clone(), amount1),
        );

        *user_pool_shares = 0;

        pool.total_shares -= shares;
        pool.reserve0 -= amount0;
        pool.reserve1 -= amount1;

        state.pools.insert(pool.id, pool);

        Self::save(state, shares);
    }

    pub fn swap(input: Swap) {
        let mut state = Self::load();

        let caller = env::caller();
        let mut pool = state
            .pools
            .get(&input.pool_id)
            .expect("Pool not found")
            .clone();

        let token0_id =
            AccountId::MultiVm(MultiVmAccountId::try_from(pool.token0.address.clone()).unwrap());
        let token1_id =
            AccountId::MultiVm(MultiVmAccountId::try_from(pool.token1.address.clone()).unwrap());

        let (reserve_in, reserve_out, amount_in, token_in, token_out) = if input.amount0_in > 0 {
            (pool.reserve0, pool.reserve1, input.amount0_in, token0_id, token1_id)
        } else {
            (pool.reserve1, pool.reserve0, input.amount1_in, token1_id, token0_id)
        };

        let amount_out = reserve_out * amount_in / (reserve_in + amount_in);

        let _commitment = env::cross_contract_call(
            token_in,
            "transfer_from".to_string(),
            0,
            &(caller.clone(), amount_in),
        );

        let _commitment = env::cross_contract_call(
            token_out,
            "transfer".to_string(),
            0,
            &(caller.clone(), amount_out),
        );

        if input.amount0_in > 0 {
            pool.reserve0 += amount_in;
            pool.reserve1 -= amount_out;
        } else {
            pool.reserve0 -= amount_out;
            pool.reserve1 += amount_in;
        }

        state.pools.insert(pool.id, pool);

        Self::save(state, ());
    }
}

impl AmmContract {
    fn load() -> State {
        env::get_storage("root".to_string()).unwrap_or_default()
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
