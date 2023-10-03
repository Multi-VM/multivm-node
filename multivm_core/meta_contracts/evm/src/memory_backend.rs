use borsh::{BorshDeserialize, BorshSerialize};
use evm::backend::{Apply, ApplyBackend, Backend, Basic, Log, MemoryVicinity};
use primitive_types::{H160, H256, U256};

use multivm_sdk::env;

use std::collections::BTreeMap;

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
        let basic = env::get_storage::<EvmBasic>(format!("basic_{:?}", address));
        basic.is_some()
    }

    fn basic(&self, address: H160) -> Basic {
        let basic = env::get_storage::<EvmBasic>(format!("basic_{:?}", address));
        basic.unwrap_or_default().into()
    }

    fn code(&self, address: H160) -> Vec<u8> {
        let code: Vec<u8> = env::get_storage(format!("code_{:?}", address)).unwrap_or_default();
        code
    }

    fn storage(&self, address: H160, index: H256) -> H256 {
        let bytes: Vec<u8> = env::get_storage(format!("storage_{:?}", address)).unwrap();
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
                        env::set_storage(format!("basic_{:?}", address), EvmBasic::from(basic));

                        if let Some(code) = code {
                            env::set_storage(format!("code_{:?}", address), code);
                        }

                        if reset_storage {
                            let empty_state = BTreeMap::<H256, H256>::new();
                            env::set_storage(
                                format!("storage_{:?}", address),
                                bincode::serialize(&empty_state).unwrap(),
                            );
                        }

                        let mut storage: BTreeMap<H256, H256> =
                            env::get_storage::<Vec<u8>>(format!("storage_{:?}", address))
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

                        env::set_storage(
                            format!("storage_{:?}", address),
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
