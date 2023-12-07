#![no_main]

use borsh::{BorshDeserialize, BorshSerialize};
use eth_primitive_types::{H160, U256};
use multivm_sdk::multivm_primitives::{EvmAddress, MultiVmAccountId};
use num::integer::Roots;
use std::{cmp::min, collections::HashMap, str::FromStr};

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
    pub executable: Option<Executable>,
    pub balance: u128,
    pub nonce: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub enum Executable {
    Evm(),
    MultiVm(MultiVmExecutable),
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct MultiVmExecutable {
    pub image_id: [u32; 8],
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
        let pools = state
            .pools
            .iter()
            .map(|(_, pool)| pool.clone())
            .collect::<Vec<_>>();
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

        let abi_bytes =
            include_bytes!("../../../multivm_core/etc/evm_contracts/erc20.abi").to_vec();
        let abi = ethabi::Contract::load(abi_bytes.as_slice()).unwrap();
        let function = abi.function("symbol").unwrap();
        let encoded_input = function.encode_input(&vec![]).unwrap();

        let token0_id = Self::token_id(input.token0.clone());
        let commitment0 =
            env::cross_contract_call_raw(token0_id, "symbol".to_string(), 0, encoded_input.clone());
        let response_bytes0: Vec<u8> = commitment0.try_deserialize_response().unwrap();
        let symbol0 = function
            .decode_output(response_bytes0.as_slice())
            .unwrap()
            .first()
            .unwrap()
            .to_string();

        let token1_id = Self::token_id(input.token1.clone());
        let commitment1 =
            env::cross_contract_call_raw(token1_id, "symbol".to_string(), 0, encoded_input.clone());
        let response_bytes1: Vec<u8> = commitment1.try_deserialize_response().unwrap();
        let symbol1 = function
            .decode_output(response_bytes1.as_slice())
            .unwrap()
            .first()
            .unwrap()
            .to_string();

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

        let contract = env::contract();
        let commitment = env::cross_contract_call(
            AccountId::system_meta_contract(),
            "account_info".to_string(),
            0,
            &contract.clone(),
        );
        let contract_account = commitment
            .try_deserialize_response::<Option<Account>>()
            .unwrap()
            .unwrap();

        let abi_bytes =
            include_bytes!("../../../multivm_core/etc/evm_contracts/erc20.abi").to_vec();
        let abi = ethabi::Contract::load(abi_bytes.as_slice()).unwrap();
        let function = abi.function("transferFrom").unwrap();

        let encoded_input0 = function
            .encode_input(&vec![
                ethabi::Token::Address(caller.evm().into()),
                ethabi::Token::Address(contract_account.evm_address.clone().into()),
                ethabi::Token::Uint(input.amount0.into()),
            ])
            .unwrap();
        let token0_id = Self::token_id(pool.token0.address.clone());
        env::cross_contract_call_raw(
            token0_id.clone(),
            "transferFrom".to_string(),
            0,
            encoded_input0.clone(),
        );

        let encoded_input1 = function
            .encode_input(&vec![
                ethabi::Token::Address(caller.evm().into()),
                ethabi::Token::Address(contract_account.evm_address.clone().into()),
                ethabi::Token::Uint(input.amount1.into()),
            ])
            .unwrap();
        let token1_id = Self::token_id(pool.token1.address.clone());
        env::cross_contract_call_raw(
            token1_id,
            "transferFrom".to_string(),
            0,
            encoded_input1.clone(),
        );

        let shares = if pool.total_shares == 0 {
            (input.amount0 * input.amount1).sqrt()
        } else {
            min(
                (U256::from(input.amount0) * U256::from(pool.total_shares)
                    / U256::from(pool.reserve0))
                .as_u128(),
                (U256::from(input.amount1) * U256::from(pool.total_shares)
                    / U256::from(pool.reserve1))
                .as_u128(),
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
        let mut pool = state.pools.get(&pool_id).expect("Pool not found").clone();
        let user_shares = state.shares.entry(caller.clone()).or_insert(HashMap::new());
        let user_pool_shares = user_shares.entry(pool.id).or_insert(0);
        let shares = *user_pool_shares;

        let amount0 = (U256::from(shares) * U256::from(pool.reserve0)
            / U256::from(pool.total_shares))
        .as_u128();
        let amount1 = (U256::from(shares) * U256::from(pool.reserve1)
            / U256::from(pool.total_shares))
        .as_u128();

        let abi_bytes =
            include_bytes!("../../../multivm_core/etc/evm_contracts/erc20.abi").to_vec();
        let abi = ethabi::Contract::load(abi_bytes.as_slice()).unwrap();
        let function = abi.function("transfer").unwrap();

        let encoded_input0 = function
            .encode_input(&vec![
                ethabi::Token::Address(caller.evm().into()),
                ethabi::Token::Uint(amount0.into()),
            ])
            .unwrap();
        let token0_id = Self::token_id(pool.token0.address.clone());
        env::cross_contract_call_raw(
            token0_id.clone(),
            "transfer".to_string(),
            0,
            encoded_input0.clone(),
        );

        let encoded_input1 = function
            .encode_input(&vec![
                ethabi::Token::Address(caller.evm().into()),
                ethabi::Token::Uint(amount1.into()),
            ])
            .unwrap();
        let token1_id = Self::token_id(pool.token1.address.clone());
        env::cross_contract_call_raw(token1_id, "transfer".to_string(), 0, encoded_input1.clone());

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

        let token0_id = Self::token_id(pool.token0.address.clone());
        let token1_id = Self::token_id(pool.token1.address.clone());

        let (reserve_in, reserve_out, amount_in, token_in, token_out) = if input.amount0_in > 0 {
            (
                pool.reserve0,
                pool.reserve1,
                input.amount0_in,
                token0_id,
                token1_id,
            )
        } else {
            (
                pool.reserve1,
                pool.reserve0,
                input.amount1_in,
                token1_id,
                token0_id,
            )
        };

        let amount_out = (U256::from(reserve_out) * U256::from(amount_in)
            / (U256::from(reserve_in + amount_in)))
        .as_u128();

        let abi_bytes =
            include_bytes!("../../../multivm_core/etc/evm_contracts/erc20.abi").to_vec();
        let abi = ethabi::Contract::load(abi_bytes.as_slice()).unwrap();

        let contract = env::contract();
        let commitment = env::cross_contract_call(
            AccountId::system_meta_contract(),
            "account_info".to_string(),
            0,
            &contract.clone(),
        );
        let contract_account = commitment
            .try_deserialize_response::<Option<Account>>()
            .unwrap()
            .unwrap();

        let transfer_from_function = abi.function("transferFrom").unwrap();
        let encoded_input0 = transfer_from_function
            .encode_input(&vec![
                ethabi::Token::Address(caller.evm().into()),
                ethabi::Token::Address(contract_account.evm_address.clone().into()),
                ethabi::Token::Uint(amount_in.into()),
            ])
            .unwrap();
        env::cross_contract_call_raw(
            token_in.clone(),
            "transferFrom".to_string(),
            0,
            encoded_input0.clone(),
        );

        let transfer_function = abi.function("transfer").unwrap();
        let encoded_input1 = transfer_function
            .encode_input(&vec![
                ethabi::Token::Address(caller.evm().into()),
                ethabi::Token::Uint(amount_out.into()),
            ])
            .unwrap();
        env::cross_contract_call_raw(
            token_out.clone(),
            "transfer".to_string(),
            0,
            encoded_input1.clone(),
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

    fn token_id(name: String) -> AccountId {
        if let Ok(h160) = H160::from_str(name.as_str()) {
            AccountId::Evm(EvmAddress::try_from(h160).unwrap())
        } else {
            AccountId::MultiVm(MultiVmAccountId::try_from(name.clone()).unwrap())
        }
    }
}

impl Pool {
    pub fn symbol(&self) -> String {
        format!("{}/{}", self.token0.symbol, self.token1.symbol).to_string()
    }
}
