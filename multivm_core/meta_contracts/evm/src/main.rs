#![no_main]

use core::panic;
use evm::backend::MemoryVicinity;
use evm::executor::stack::{MemoryStackState, StackExecutor, StackSubstateMetadata};
use evm::{backend::ApplyBackend, Config};
use memory_backend::{EvmBasic, EvmMemoryBackend};
use primitive_types::{H160, H256, U256};
use std::collections::BTreeMap;

mod memory_backend;

struct Contract;

#[multivm_sdk_macros::contract]
impl Contract {
    pub fn init() {
        for user in ["alice", "bob", "charlie", "eve"] {
            let address = env::get_evm_address(AccountId::from(format!("{}.multivm", user)));
            let basic = EvmBasic {
                balance: U256::from(10_000_000_000_000_000_000_000_000u128).into(),
                nonce: U256::one().into(),
            };

            let storage = BTreeMap::<H256, H256>::new();

            env::set_storage(format!("basic_{:?}", address), basic);
            env::set_storage(
                format!("storage_{:?}", address),
                bincode::serialize(&storage).unwrap(),
            );
        }

        env::commit(())
    }

    pub fn deploy_contract(code: Vec<u8>) {
        let config = Config::istanbul();

        let vicinity = MemoryVicinity {
            gas_price: U256::zero(),
            origin: H160::default(),
            block_hashes: Vec::new(),
            block_number: Default::default(),
            block_coinbase: Default::default(),
            block_timestamp: Default::default(),
            block_difficulty: Default::default(),
            block_gas_limit: Default::default(),
            chain_id: U256::one(),
            block_base_fee_per_gas: U256::zero(),
            block_randomness: None,
        };

        let mut backend = EvmMemoryBackend::new(&vicinity);
        let metadata = StackSubstateMetadata::new(u64::MAX, &config);
        let state = MemoryStackState::new(metadata, &backend);
        let precompiles = BTreeMap::new();
        let mut executor = StackExecutor::new_with_precompiles(state, &config, &precompiles);

        let caller = env::caller();
        let caller_address = env::get_evm_address(caller);

        let token_address = executor.create_address(evm::CreateScheme::Legacy {
            caller: caller_address,
        });

        let reason =
            executor.transact_create(caller_address, U256::from(0), code, u64::MAX, Vec::new());

        let s = executor.into_state();
        let (a, b) = s.deconstruct();
        backend.apply(a, b, false);

        env::commit(((token_address.to_fixed_bytes()), reason.1));
    }

    pub fn call_contract(input: ([u8; 20], Vec<u8>)) {
        let config = Config::istanbul();

        let vicinity = MemoryVicinity {
            gas_price: U256::zero(),
            origin: H160::default(),
            block_hashes: Vec::new(),
            block_number: Default::default(),
            block_coinbase: Default::default(),
            block_timestamp: Default::default(),
            block_difficulty: Default::default(),
            block_gas_limit: Default::default(),
            chain_id: U256::one(),
            block_base_fee_per_gas: U256::zero(),
            block_randomness: None,
        };

        let caller = env::caller();
        let caller_address = env::get_evm_address(caller);

        let mut backend = EvmMemoryBackend::new(&vicinity);
        let metadata = StackSubstateMetadata::new(u64::MAX, &config);
        let state = MemoryStackState::new(metadata, &backend);
        let precompiles = BTreeMap::new();
        let mut executor = StackExecutor::new_with_precompiles(state, &config, &precompiles);

        let address = H160::from_slice(&input.0);
        let data = input.1;

        let reason = executor.transact_call(
            caller_address,
            address,
            U256::zero(),
            data,
            u64::MAX,
            Vec::new(),
        );

        let s = executor.into_state();
        let (a, b) = s.deconstruct();
        backend.apply(a, b, false);

        env::commit(reason.1);
    }
}
