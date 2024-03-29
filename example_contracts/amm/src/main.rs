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
    pub decimals: u8,
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

const ABI_BYTES: &[u8] = include_bytes!("../../../multivm_core/etc/evm_contracts/erc20.abi");

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

        let abi = ethabi::Contract::load(ABI_BYTES).unwrap();
        let symbol_function = abi.function("symbol").unwrap();
        let symbol_input = symbol_function.encode_input(&vec![]).unwrap();
        let decimals_function = abi.function("decimals").unwrap();
        let decimals_input = decimals_function.encode_input(&vec![]).unwrap();

        let token0_id = Self::token_id(input.token0.clone());
        let commitment = env::cross_contract_call_raw(
            token0_id.clone(),
            "symbol".to_string(),
            0,
            symbol_input.clone(),
        );
        let response_bytes: Vec<u8> = borsh::from_slice(&commitment.response.unwrap()).unwrap();
        let symbol0 = symbol_function
            .decode_output(response_bytes.as_slice())
            .unwrap()
            .first()
            .unwrap()
            .to_string();

        let commitment = env::cross_contract_call_raw(
            token0_id.clone(),
            "decimals".to_string(),
            0,
            decimals_input.clone(),
        );
        let response_bytes: Vec<u8> = borsh::from_slice(&commitment.response.unwrap()).unwrap();
        let decimals0 = decimals_function
            .decode_output(response_bytes.as_slice())
            .unwrap()
            .first()
            .unwrap()
            .clone()
            .into_uint()
            .unwrap()
            .try_into()
            .unwrap();

        let token1_id = Self::token_id(input.token1.clone());
        let commitment = env::cross_contract_call_raw(
            token1_id.clone(),
            "symbol".to_string(),
            0,
            symbol_input.clone(),
        );
        let response_bytes: Vec<u8> = borsh::from_slice(&commitment.response.unwrap()).unwrap();
        let symbol1 = symbol_function
            .decode_output(response_bytes.as_slice())
            .unwrap()
            .first()
            .unwrap()
            .to_string();

        let commitment = env::cross_contract_call_raw(
            token1_id.clone(),
            "decimals".to_string(),
            0,
            decimals_input.clone(),
        );
        let response_bytes: Vec<u8> = borsh::from_slice(&commitment.response.unwrap()).unwrap();
        let decimals1 = decimals_function
            .decode_output(response_bytes.as_slice())
            .unwrap()
            .first()
            .unwrap()
            .clone()
            .into_uint()
            .unwrap()
            .try_into()
            .unwrap();

        let token0 = Token {
            symbol: symbol0,
            address: input.token0,
            decimals: decimals0,
        };
        let token1 = Token {
            symbol: symbol1,
            address: input.token1,
            decimals: decimals1,
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

        let (amount0, amount1) = if pool.total_shares == 0 {
            (input.amount0, input.amount1)
        } else {
            assert!(
                (input.amount0 != 0) ^ (input.amount1 != 0),
                "You need to specify the amount for only one token {} {}",
                input.amount0,
                input.amount1
            );

            if input.amount0 > 0 {
                (
                    input.amount0,
                    (U256::from(input.amount0) * U256::from(pool.reserve1)
                        / U256::from(pool.reserve0))
                    .as_u128(),
                )
            } else {
                (
                    (U256::from(input.amount1) * U256::from(pool.reserve0)
                        / U256::from(pool.reserve1))
                    .as_u128(),
                    input.amount1,
                )
            }
        };

        let contract = env::contract();
        let commitment = env::cross_contract_call(
            AccountId::system_meta_contract(),
            "account_info".to_string(),
            0,
            &contract.clone(),
        );

        let contract_account: Option<Account> =
            borsh::from_slice(&commitment.response.unwrap()).unwrap();

        let abi = ethabi::Contract::load(ABI_BYTES).unwrap();
        let function = abi.function("transferFrom").unwrap();

        let encoded_input0 = function
            .encode_input(&vec![
                ethabi::Token::Address(caller.evm().into()),
                ethabi::Token::Address(
                    contract_account.clone().unwrap().evm_address.clone().into(),
                ),
                ethabi::Token::Uint(amount0.into()),
            ])
            .unwrap();
        let token0_id = Self::token_id(pool.token0.address.clone());
        let commitment = env::cross_contract_call_raw(
            token0_id.clone(),
            "transferFrom".to_string(),
            0,
            encoded_input0.clone(),
        );
        if commitment.response.is_err() {
            panic!("Can not transfer token {}", token0_id);
        }

        let encoded_input1 = function
            .encode_input(&vec![
                ethabi::Token::Address(caller.evm().into()),
                ethabi::Token::Address(contract_account.unwrap().evm_address.clone().into()),
                ethabi::Token::Uint(amount1.into()),
            ])
            .unwrap();
        let token1_id = Self::token_id(pool.token1.address.clone());
        let commitment = env::cross_contract_call_raw(
            token1_id.clone(),
            "transferFrom".to_string(),
            0,
            encoded_input1.clone(),
        );
        if commitment.response.is_err() {
            panic!("Can not transfer token {}", token1_id);
        }

        let shares = if pool.total_shares == 0 {
            (amount0 * amount1).sqrt()
        } else {
            (U256::from(amount0) * U256::from(pool.total_shares) / U256::from(pool.reserve0))
                .as_u128()
        };

        pool.total_shares += shares;
        pool.reserve0 += amount0;
        pool.reserve1 += amount1;

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

        let abi = ethabi::Contract::load(ABI_BYTES).unwrap();
        let function = abi.function("transfer").unwrap();

        let encoded_input0 = function
            .encode_input(&vec![
                ethabi::Token::Address(caller.evm().into()),
                ethabi::Token::Uint(amount0.into()),
            ])
            .unwrap();
        let token0_id = Self::token_id(pool.token0.address.clone());
        let commitment = env::cross_contract_call_raw(
            token0_id.clone(),
            "transfer".to_string(),
            0,
            encoded_input0.clone(),
        );
        if commitment.response.is_err() {
            panic!("Can not send token {}", token0_id);
        }

        let encoded_input1 = function
            .encode_input(&vec![
                ethabi::Token::Address(caller.evm().into()),
                ethabi::Token::Uint(amount1.into()),
            ])
            .unwrap();
        let token1_id = Self::token_id(pool.token1.address.clone());
        let commitment = env::cross_contract_call_raw(
            token1_id,
            "transfer".to_string(),
            0,
            encoded_input1.clone(),
        );
        if commitment.response.is_err() {
            panic!("Can not send token {}", token0_id);
        }

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

        let abi = ethabi::Contract::load(ABI_BYTES).unwrap();

        let contract = env::contract();
        let commitment = env::cross_contract_call(
            AccountId::system_meta_contract(),
            "account_info".to_string(),
            0,
            &contract.clone(),
        );

        let contract_account: Option<Account> =
            borsh::from_slice(&commitment.response.unwrap()).unwrap();

        let transfer_from_function = abi.function("transferFrom").unwrap();
        let encoded_input0 = transfer_from_function
            .encode_input(&vec![
                ethabi::Token::Address(caller.evm().into()),
                ethabi::Token::Address(contract_account.unwrap().evm_address.clone().into()),
                ethabi::Token::Uint(amount_in.into()),
            ])
            .unwrap();
        let commitment = env::cross_contract_call_raw(
            token_in.clone(),
            "transferFrom".to_string(),
            0,
            encoded_input0.clone(),
        );
        if commitment.response.is_err() {
            panic!("Can not transfer token {}", token_in);
        }

        let transfer_function = abi.function("transfer").unwrap();
        let encoded_input1 = transfer_function
            .encode_input(&vec![
                ethabi::Token::Address(caller.evm().into()),
                ethabi::Token::Uint(amount_out.into()),
            ])
            .unwrap();
        let commitment = env::cross_contract_call_raw(
            token_out.clone(),
            "transfer".to_string(),
            0,
            encoded_input1.clone(),
        );
        if commitment.response.is_err() {
            panic!("Can not send token {}", token_out);
        }

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
