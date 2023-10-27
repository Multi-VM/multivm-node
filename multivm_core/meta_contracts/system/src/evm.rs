use crate::{
    account_management::{self, Account},
    system_env,
};
use eth_primitive_types::{H160, H256, U256};
use evm::backend::MemoryVicinity;
use evm::executor::stack::{MemoryStackState, StackExecutor, StackSubstateMetadata};
use evm::{backend::ApplyBackend, Config};
use multivm_primitives::EvmAddress;
use std::collections::BTreeMap;

pub fn deploy_evm_contract(owner: Account, code: Vec<u8>) {
    let config = Config::istanbul();

    let vicinity = MemoryVicinity {
        // TODO: set these values
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

    let owner_address = owner.evm_address.into();

    let token_address = executor.create_address(evm::CreateScheme::Legacy {
        caller: owner_address,
    });

    let reason = executor.transact_create(owner_address, U256::from(0), code, u64::MAX, Vec::new());

    let s = executor.into_state();
    let (a, b) = s.deconstruct();
    backend.apply(a, b, false);

    system_env::commit(((token_address.to_fixed_bytes()), reason.1));
}

pub fn call_contract(
    caller: Account,
    contract_address: EvmAddress,
    data: Vec<u8>,
    apply_changes: bool,
) {
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

    let reason = executor.transact_call(
        caller.evm_address.into(),
        contract_address.into(),
        U256::zero(),
        data,
        u64::MAX,
        Vec::new(),
    );

    let s = executor.into_state();
    let (a, b) = s.deconstruct();
    if apply_changes {
        backend.apply(a, b, false);
    }

    system_env::commit(reason.1);
}

use borsh::{BorshDeserialize, BorshSerialize};
use evm::backend::{Apply, Backend, Basic, Log};

#[derive(Clone, Debug)]
pub struct EvmMemoryBackend<'vicinity> {
    vicinity: &'vicinity MemoryVicinity,
    logs: Vec<Log>,
}

impl<'vicinity> EvmMemoryBackend<'vicinity> {
    pub fn new(vicinity: &'vicinity MemoryVicinity) -> Self {
        Self {
            vicinity,
            logs: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct EvmBasic {
    pub balance: [u8; 32],
    pub nonce: [u8; 32],
}

impl From<Basic> for EvmBasic {
    fn from(basic: Basic) -> Self {
        Self {
            balance: basic.balance.into(),
            nonce: basic.nonce.into(),
        }
    }
}

impl From<EvmBasic> for Basic {
    fn from(basic: EvmBasic) -> Self {
        Self {
            balance: basic.balance.into(),
            nonce: basic.nonce.into(),
        }
    }
}

const CODE_KEY: &str = "evm_code";
const STORAGE_KEY: &str = "evm_storage";

impl<'vicinity> Backend for EvmMemoryBackend<'vicinity> {
    fn gas_price(&self) -> U256 {
        self.vicinity.gas_price
    }
    fn origin(&self) -> H160 {
        self.vicinity.origin
    }
    fn block_hash(&self, number: U256) -> H256 {
        if number >= self.vicinity.block_number
            || self.vicinity.block_number - number - U256::one()
                >= U256::from(self.vicinity.block_hashes.len())
        {
            H256::default()
        } else {
            let index = (self.vicinity.block_number - number - U256::one()).as_usize();
            self.vicinity.block_hashes[index]
        }
    }
    fn block_number(&self) -> U256 {
        self.vicinity.block_number
    }
    fn block_coinbase(&self) -> H160 {
        self.vicinity.block_coinbase
    }
    fn block_timestamp(&self) -> U256 {
        self.vicinity.block_timestamp
    }
    fn block_difficulty(&self) -> U256 {
        self.vicinity.block_difficulty
    }
    fn block_randomness(&self) -> Option<H256> {
        self.vicinity.block_randomness
    }
    fn block_gas_limit(&self) -> U256 {
        self.vicinity.block_gas_limit
    }
    fn block_base_fee_per_gas(&self) -> U256 {
        self.vicinity.block_base_fee_per_gas
    }

    fn chain_id(&self) -> U256 {
        self.vicinity.chain_id
    }

    fn exists(&self, address: H160) -> bool {
        account_management::account_exists(&EvmAddress::from(address).into())
    }

    fn basic(&self, address: H160) -> Basic {
        let (balance, nonce) = match account_management::account(&EvmAddress::from(address).into())
            .map(|account| (account.balance.into(), account.nonce.into()))
        {
            Some((balance, nonce)) => (Some(balance), Some(nonce)),
            None => (None, None),
        };

        Basic {
            balance: balance.unwrap_or_default(),
            nonce: nonce.unwrap_or_default(),
        }
    }

    fn code(&self, address: H160) -> Vec<u8> {
        account_management::account_storage(&EvmAddress::from(address).into(), CODE_KEY.into())
            .unwrap_or_default()
    }

    fn storage(&self, address: H160, index: H256) -> H256 {
        let bytes: Vec<u8> = account_management::account_storage(
            &EvmAddress::from(address).into(),
            STORAGE_KEY.into(),
        )
        .unwrap_or_default();

        let storage: BTreeMap<H256, H256> = bincode::deserialize(&bytes).unwrap();

        storage.get(&index).cloned().unwrap_or_default()
    }

    fn original_storage(&self, address: H160, index: H256) -> Option<H256> {
        Some(self.storage(address, index))
    }
}

impl<'vicinity> ApplyBackend for EvmMemoryBackend<'vicinity> {
    fn apply<A, I, L>(&mut self, values: A, logs: L, delete_empty: bool)
    where
        A: IntoIterator<Item = Apply<I>>,
        I: IntoIterator<Item = (H256, H256)>,
        L: IntoIterator<Item = Log>,
    {
        for apply in values {
            match apply {
                Apply::Modify {
                    address,
                    basic,
                    code,
                    storage: new_storage,
                    reset_storage,
                } => {
                    let is_empty = {
                        let mut account =
                            account_management::account(&EvmAddress::from(address).into())
                                .unwrap_or_else(|| {
                                    account_management::Account::try_create_empty_evm(
                                        address.into(),
                                    )
                                });
                        account.balance = basic.balance.as_u128();
                        account.nonce = basic.nonce.as_u64();
                        account_management::update_account(account);

                        if let Some(code) = code {
                            account_management::update_account_storage(
                                &EvmAddress::from(address).into(),
                                CODE_KEY.into(),
                                code,
                            );
                        }

                        if reset_storage {
                            let empty_state = BTreeMap::<H256, H256>::new();
                            account_management::update_account_storage(
                                &EvmAddress::from(address).into(),
                                STORAGE_KEY.into(),
                                bincode::serialize(&empty_state).unwrap(),
                            );
                        }

                        let mut storage: BTreeMap<H256, H256> =
                            account_management::account_storage::<Vec<u8>>(
                                &EvmAddress::from(address).into(),
                                STORAGE_KEY.into(),
                            )
                            .map(|bytes| bincode::deserialize(&bytes).unwrap())
                            .unwrap_or_default();

                        let zeros = storage
                            .iter()
                            .filter(|(_, v)| v == &&H256::default())
                            .map(|(k, _)| *k)
                            .collect::<Vec<H256>>();

                        for zero in zeros {
                            storage.remove(&zero);
                        }

                        for (index, value) in new_storage {
                            if value == H256::default() {
                                storage.remove(&index);
                            } else {
                                storage.insert(index, value);
                            }
                        }

                        account_management::update_account_storage(
                            &EvmAddress::from(address).into(),
                            STORAGE_KEY.into(),
                            bincode::serialize(&storage).unwrap(),
                        );

                        // TODO
                        // account.balance == U256::zero()
                        //     && account.nonce == U256::zero()
                        //     && account.code.is_empty()
                        false
                    };

                    if is_empty && delete_empty {
                        // TODO
                        // self.state.remove(&address);
                        unimplemented!("delete_empty");
                    }
                }
                Apply::Delete { address: _ } => {
                    // TODO
                    // self.state.remove(&address);
                    unimplemented!("delete");
                }
            }
        }

        for log in logs {
            self.logs.push(log);
        }
    }
}
