use anyhow::{Context, Result};
use borsh::BorshDeserialize;
use risc0_zkvm::sha::rust_crypto::{Digest, Sha256};
use tracing::{debug, info, span, Level};

use multivm_primitives::{
    syscalls::{
        CrossContractCallRequest, GetStorageResponse, SetStorageRequest, CROSS_CONTRACT_CALL,
        GET_STORAGE_CALL, SET_STORAGE_CALL,
    },
    AccountId, ContractCallContext,
};

use crate::{
    account::Executable, bootstraper::Action, outcome::ExecutionOutcome, utils, viewer::Viewer,
};

use std::{cell::RefCell, rc::Rc};

const MAX_MEMORY: u32 = 0x10000000;
const PAGE_SIZE: u32 = 0x400;

pub struct Executor {
    context: ContractCallContext,
    db: sled::Db,
    cross_calls_outcomes: Rc<RefCell<Vec<ExecutionOutcome>>>,
}

impl Executor {
    pub fn new(context: ContractCallContext, db: sled::Db) -> Self {
        Self {
            context,
            db,
            cross_calls_outcomes: Default::default(),
        }
    }

    pub fn execute(self) -> Result<ExecutionOutcome> {
        let contract_id = self.context.contract_id.clone();
        let (call_bytes, elf) = if contract_id != AccountId::system_meta_contract() {
            let contract = Viewer::account_info(&contract_id, self.db.clone())
                .context(contract_id.clone())
                .expect("Loading storage for non-existent contract");

            match contract.executable {
                Some(Executable::MultiVm(_)) => {
                    let elf = self
                        .load_contract(contract_id.into())
                        .context(format!("Load contract {:?}", self.context.contract_id))
                        .unwrap();
                    (borsh::to_vec(&self.context).unwrap(), elf)
                }
                Some(Executable::Evm()) => {
                    let call = Action::EvmCall(self.context.clone());
                    (
                        borsh::to_vec(&call).unwrap(),
                        meta_contracts::SYSTEM_META_CONTRACT_ELF.to_vec(),
                    )
                }
                None => unreachable!("Account is non-executable"),
            }
        } else {
            (
                borsh::to_vec(&Action::Call(self.context.clone())).unwrap(),
                meta_contracts::SYSTEM_META_CONTRACT_ELF.to_vec(),
            )
        };

        let env = risc0_zkvm::ExecutorEnv::builder()
            .write_slice(&call_bytes)
            .session_limit(Some(u64::MAX))
            .io_callback(CROSS_CONTRACT_CALL, self.callback_on_cross_contract_call())
            .io_callback(GET_STORAGE_CALL, self.callback_on_get_storage())
            .io_callback(SET_STORAGE_CALL, self.callback_on_set_storage())
            .stdout(ContractLogger::new(self.context.contract_id.clone()))
            .build()
            .unwrap();

        info!(contract = ?self.context.contract_id, size = ?elf.len(), "Executing contract");

        let program = risc0_zkvm::Program::load_elf(&elf, MAX_MEMORY).unwrap();
        let image = risc0_zkvm::MemoryImage::new(&program, PAGE_SIZE).unwrap();
        let exec = risc0_zkvm::default_executor();

        let session = exec.execute(env, image).unwrap();

        Ok(ExecutionOutcome::new(
            session,
            0,
            self.cross_calls_outcomes.take(),
        ))
    }

    fn load_contract(&self, contract_id: AccountId) -> Result<Vec<u8>> {
        let db_key = format!("contracts_code.{}", contract_id.to_string());
        info!(db_key, "load contract");

        let code = self
            .db
            .get(db_key)
            .expect("Failed to get storage from db")
            .map(|v| v.to_vec())
            .expect("Contract not found");

        Ok(code)
    }

    pub fn callback_on_cross_contract_call<'a>(
        &'a self,
    ) -> impl Fn(risc0_zkvm::Bytes) -> risc0_zkvm::Result<risc0_zkvm::Bytes> + 'a {
        |from_guest| {
            debug!("Handling syscall for cross contract call");

            let req: CrossContractCallRequest = BorshDeserialize::try_from_slice(&from_guest)
                .expect("Invalid contract call request");

            let call_context = ContractCallContext {
                contract_id: req.contract_id,
                contract_call: req.contract_call,
                sender_id: self.context.contract_id.clone(),
                signer_id: self.context.signer_id.clone(),
                environment: self.context.environment.clone(),
            };

            debug!(call_context=?call_context, "Executing cross contract call");

            let outcome = Executor::new(call_context, self.db.clone())
                .execute()
                .context("Cross Contract Call failed")?;

            let commitment = borsh::to_vec(&outcome.commitment).unwrap();

            (self.cross_calls_outcomes.borrow_mut()).push(outcome);

            Ok(commitment.into())
        }
    }

    pub fn callback_on_get_storage<'a>(
        &'a self,
    ) -> impl Fn(risc0_zkvm::Bytes) -> risc0_zkvm::Result<risc0_zkvm::Bytes> + 'a {
        |from_guest| {
            let span = span!(Level::DEBUG, "get_storage call handler");
            let _enter = span.enter();

            let key = String::from_utf8(from_guest.into()).unwrap();

            let storage_location = if self.context.contract_id == AccountId::system_meta_contract()
            {
                AccountId::system_meta_contract()
            } else {
                let contract = Viewer::account_info(&self.context.contract_id, self.db.clone())
                    .expect("Loading storage for non-existent contract");

                match contract.executable {
                    Some(Executable::MultiVm(_)) => contract
                        .multivm_account_id
                        .expect("Contract without MultiVmAccountId")
                        .into(),
                    Some(Executable::Evm()) => AccountId::system_meta_contract(),
                    None => unreachable!("Loading storage for non-executable account"),
                }
            };

            let db_key = format!("committed_storage.{}.{}", storage_location, key);

            let storage = self
                .db
                .get(db_key)
                .expect("Failed to get storage from db")
                .map(|v| v.to_vec());

            // let hash = {
            //     let algorithm = &mut Sha256::default();
            //     algorithm.update(&storage);
            //     algorithm.finalize_reset().as_slice().to_vec()
            // }

            let response = GetStorageResponse { storage };

            let response_bytes = borsh::to_vec(&response).unwrap();

            debug!(contract=?storage_location, key=?key, "Loading storage");

            Ok(response_bytes.into())
        }
    }

    pub fn callback_on_set_storage<'a>(
        &'a self,
    ) -> impl Fn(risc0_zkvm::Bytes) -> risc0_zkvm::Result<risc0_zkvm::Bytes> + 'a {
        |from_guest| {
            let span = span!(Level::DEBUG, "set_storage call handler");
            let _enter = span.enter();

            let request: SetStorageRequest = BorshDeserialize::try_from_slice(&from_guest).unwrap();

            let algorithm = &mut Sha256::default();
            algorithm.update(request.storage.clone());
            let hash2 = algorithm.finalize_reset();
            assert_eq!(request.hash, hash2.as_slice());

            let storage_location = if self.context.contract_id == AccountId::system_meta_contract()
            {
                AccountId::system_meta_contract()
            } else {
                let contract = Viewer::account_info(&self.context.contract_id, self.db.clone())
                    .expect("Loading storage for non-existent contract");

                match contract.executable {
                    Some(Executable::MultiVm(_)) => contract
                        .multivm_account_id
                        .expect("Contract without MultiVmAccountId")
                        .into(),
                    Some(Executable::Evm()) => AccountId::system_meta_contract(),
                    None => unreachable!("Loading storage for non-executable account"),
                }
            };

            debug!(contract=?storage_location, key=?request.key, new_hash = utils::bytes_to_hex(hash2.as_slice()), "Updating storage");

            let db_key = format!("committed_storage.{}.{}", storage_location, request.key);

            self.db
                .insert(db_key, request.storage)
                .expect("Failed to insert storage to db");

            Ok(Default::default())
        }
    }
}

pub struct ContractLogger {
    pub contract_id: AccountId,
}

impl ContractLogger {
    pub fn new(contract_id: AccountId) -> Self {
        Self { contract_id }
    }
}

impl std::io::Write for ContractLogger {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // TODO: handle non-utf8 logs
        let msg = String::from_utf8(buf.to_vec()).unwrap();

        tracing::info!(contract_id = ?self.contract_id, msg, "📜 Contract log");

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        unimplemented!()
    }
}
