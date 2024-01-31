use borsh::{BorshDeserialize, BorshSerialize};
use once_cell::sync::Lazy;
use risc0_zkvm::sha::{Impl as HashImpl, Sha256};
use std::{collections::HashMap, sync::Mutex};

use multivm_primitives::{
    syscalls::{
        CrossContractCallRequest, DeployContractRequest, GetStorageResponse, SetStorageRequest,
        CROSS_CONTRACT_CALL, DEPLOY_CONTRACT_CALL, GET_STORAGE_CALL, SET_STORAGE_CALL,
    },
    AccountId, Commitment, ContractCall, ContractCallContext, ContractError, Digest as HashDigest,
    Event, StorageKey, SystemEvent,
};

pub fn setup_env(call: &ContractCallContext) {
    let mut env = ENV.lock().unwrap();
    *env = Some(Env::new_from_call(call));

    std::panic::set_hook(Box::new(|i| abort(i.to_string())));
}

#[derive(Debug, BorshSerialize)]
struct Env {
    signer_id: AccountId,
    caller_id: AccountId,
    contract_id: AccountId,
    gas: u64,

    call_hash: HashDigest,
    initial_storage_hashes: HashMap<StorageKey, HashDigest>,
    storage_cache: HashMap<StorageKey, (Vec<u8>, bool)>,
    cross_calls_hashes: Vec<(HashDigest, HashDigest)>,
    events: Vec<Event>,
}

impl Env {
    fn new_from_call(call: &ContractCallContext) -> Self {
        let call_bytes = borsh::to_vec(&call).expect("Expected to serialize");
        let call_hash = HashImpl::hash_bytes(&call_bytes)
            .to_owned()
            .as_bytes()
            .try_into()
            .unwrap();

        Self {
            signer_id: call.signer_id.clone(),
            caller_id: call.sender_id.clone(),
            contract_id: call.contract_id.clone(),
            gas: call.contract_call.gas,
            call_hash: call_hash,
            initial_storage_hashes: Default::default(),
            storage_cache: Default::default(),
            cross_calls_hashes: Default::default(),
            events: Default::default(),
        }
    }
}

static ENV: Lazy<Mutex<Option<Env>>> = Lazy::new(|| Mutex::new(None));

impl Env {
    /// Returns the current call signer
    pub fn signer(&self) -> AccountId {
        self.signer_id.clone()
    }

    /// Returns the current call sender
    pub fn caller(&self) -> AccountId {
        self.caller_id.clone()
    }

    /// Returns the current call contract
    pub fn contract(&self) -> AccountId {
        self.contract_id.clone()
    }

    /// Makes a cross-contract call
    pub fn cross_contract_call(&mut self, req: CrossContractCallRequest) -> Commitment {
        let req_bytes = borsh::to_vec(&req).expect("Expected to serialize");
        let req_hash = HashImpl::hash_bytes(&req_bytes)
            .to_owned()
            .as_bytes()
            .try_into()
            .unwrap();

        let response = risc0_zkvm::guest::env::send_recv_slice(
            CROSS_CONTRACT_CALL,
            borsh::to_vec(&req).unwrap().as_slice(),
        )
        .to_vec();

        let commitment = Commitment::try_from_bytes(response).expect("Commitment is corrupted");

        // assert_eq!(req_hash, commitment.call_hash); // TODO: fix

        let response_bytes = borsh::to_vec(&commitment.response).expect("Expected to serialize");

        let output_hash = HashImpl::hash_bytes(&response_bytes)
            .to_owned()
            .as_bytes()
            .try_into()
            .unwrap();

        self.cross_calls_hashes.push((req_hash, output_hash));

        commitment
    }

    /// Returns the storage value for the given key, return None if storage is not exist
    pub fn get_storage<T: BorshDeserialize>(&mut self, key: StorageKey) -> Option<T> {
        if let Some(storage_bytes) = self.storage_cache.get(&key) {
            return Some(
                BorshDeserialize::try_from_slice(storage_bytes.0.as_slice())
                    .expect("Expected to deserialize"),
            );
        }

        let response: Vec<u8> =
            risc0_zkvm::guest::env::send_recv_slice(GET_STORAGE_CALL, &key.clone().into_bytes())
                .to_vec();

        let response: GetStorageResponse =
            BorshDeserialize::try_from_slice(&mut response.as_slice())
                .expect("GetStorageResponse is corrupted");

        let Some(storage) = response.storage else {
            return None;
        };

        let hash = HashImpl::hash_bytes(&storage)
            .to_owned()
            .as_bytes()
            .try_into()
            .unwrap();

        self.storage_cache
            .insert(key.clone(), (storage.clone(), false));
        self.initial_storage_hashes.insert(key, hash);

        Some(
            borsh::BorshDeserialize::deserialize(&mut storage.as_slice())
                .expect("Expected to deserialize"),
        )
    }

    pub fn set_storage<T: borsh::BorshSerialize>(&mut self, key: String, data: T) {
        let storage_bytes = borsh::to_vec(&data).expect("Expected to serialize");

        self.storage_cache
            .insert(key.clone(), (storage_bytes.clone(), true));
    }

    fn send_storage_update(key: String, storage: Vec<u8>) -> HashDigest {
        let hash = HashImpl::hash_bytes(&storage)
            .to_owned()
            .as_bytes()
            .try_into()
            .unwrap();

        let request = SetStorageRequest {
            key: key.clone(),
            hash,
            storage,
        };

        let to_host = borsh::to_vec(&request).expect("Expected to serialize");

        let _: Vec<u8> =
            risc0_zkvm::guest::env::send_recv_slice(SET_STORAGE_CALL, &to_host).to_vec();

        hash
    }

    pub fn deploy_contract(&self, account_id: AccountId, image_id: [u32; 8]) {
        let request = DeployContractRequest {
            account_id,
            image_id,
        };

        let to_host = borsh::to_vec(&request).expect("Expected to serialize");

        let _: Vec<u8> =
            risc0_zkvm::guest::env::send_recv_slice(DEPLOY_CONTRACT_CALL, &to_host).to_vec();
    }

    // pub fn event<T: borsh::BorshSerialize>(&mut self, message: T) {
    //     let message_bytes = borsh::to_vec(&message).expect("Expected to serialize");
    //     self.event_raw(message_bytes)
    // }

    // pub fn event_raw(&mut self, message_bytes: Vec<u8>) {
    //     self.events.push(Event::ContractEvent(message_bytes))
    // }

    pub fn event_system(&mut self, event: SystemEvent) {
        self.events.push(Event::System(event));
    }

    pub fn commit<T: borsh::BorshSerialize>(self, output: T) {
        let Env {
            signer_id: _,
            caller_id: _,
            contract_id: _,
            gas: _,
            call_hash,
            initial_storage_hashes: _, // TODO: fix  storage
            storage_cache,
            cross_calls_hashes,
            events,
        } = self;

        let response = borsh::to_vec(&output).expect("Expected to serialize");

        storage_cache
            .into_iter()
            .for_each(|(key, (storage, was_changed))| {
                if was_changed {
                    Env::send_storage_update(key.clone(), storage.clone());
                }
            });

        let commitment = Commitment {
            response: Ok(response),
            call_hash: call_hash,
            cross_calls_hashes: cross_calls_hashes,
            previous_account_root: Default::default(),
            new_account_root: Default::default(),
            events,
        };

        risc0_zkvm::guest::env::commit_slice(
            &borsh::to_vec(&commitment).expect("Expected to serialize"),
        );

        risc0_zkvm::guest::env::pause(); // TODO: replace to exit
    }

    pub fn abort(self, message: String) {
        let Env {
            signer_id: _,
            caller_id: _,
            contract_id: _,
            gas: _,
            call_hash,
            initial_storage_hashes: _, // TODO: fix  storage
            storage_cache: _,
            cross_calls_hashes,
            events,
        } = self;

        let commitment = Commitment {
            response: Err(ContractError::new(message)),
            call_hash: call_hash,
            cross_calls_hashes: cross_calls_hashes,
            previous_account_root: Default::default(),
            new_account_root: Default::default(),
            events,
        };

        risc0_zkvm::guest::env::commit_slice(
            &borsh::to_vec(&commitment).expect("Expected to serialize"),
        );

        risc0_zkvm::guest::env::pause(); // TODO: replace to exit
    }
}

/// Returns the current call signer
pub fn signer() -> AccountId {
    ENV.lock().unwrap().as_ref().unwrap().signer()
}

/// Returns the current call sender
pub fn caller() -> AccountId {
    ENV.lock().unwrap().as_ref().unwrap().caller()
}

/// Returns the current call contract
pub fn contract() -> AccountId {
    ENV.lock().unwrap().as_ref().unwrap().contract()
}

/// Makes a cross-contract call with raw input
pub fn cross_contract_call_raw(
    contract_id: AccountId,
    method: String,
    gas: u64,
    args: Vec<u8>,
) -> Commitment {
    let call = ContractCall::new_raw(method, args, gas, 0);
    let req = CrossContractCallRequest::new(contract_id, call);
    ENV.lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .cross_contract_call(req.into())
}

/// Returns the storage value for the given key, return None if storage is not exist
pub fn get_storage<T: BorshDeserialize>(key: StorageKey) -> Option<T> {
    ENV.lock().unwrap().as_mut().unwrap().get_storage(key)
}

pub fn set_storage<T: borsh::BorshSerialize>(key: String, data: T) {
    ENV.lock().unwrap().as_mut().unwrap().set_storage(key, data)
}

pub fn commit<T: borsh::BorshSerialize>(output: T) {
    ENV.lock().unwrap().take().unwrap().commit(output)
}

pub fn event_system(event: SystemEvent) {
    ENV.lock().unwrap().as_mut().unwrap().event_system(event)
}

// pub fn event<T: borsh::BorshSerialize>(message: T) {
//     ENV.lock().unwrap().as_mut().unwrap().event(message)
// }

// pub fn event_raw(message: Vec<u8>) {
//     ENV.lock().unwrap().as_mut().unwrap().event_raw(message)
// }

pub fn abort(message: String) {
    ENV.lock().unwrap().take().unwrap().abort(message)
}

pub fn deploy_contract(account_id: AccountId, image_id: [u32; 8]) {
    ENV.lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .deploy_contract(account_id, image_id)
}
