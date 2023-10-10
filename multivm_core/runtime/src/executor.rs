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

use crate::{outcome::ExecutionOutcome, utils};

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

    pub fn execute(self) -> ExecutionOutcome {
        let call_bytes = borsh::to_vec(&self.context).unwrap();

        let env = risc0_zkvm::ExecutorEnv::builder()
            .add_input(&risc0_zkvm::serde::to_vec(&call_bytes).unwrap())
            .session_limit(Some(usize::MAX))
            .io_callback(CROSS_CONTRACT_CALL, self.callback_on_cross_contract_call())
            .io_callback(GET_STORAGE_CALL, self.callback_on_get_storage())
            .io_callback(SET_STORAGE_CALL, self.callback_on_set_storage())
            .stdout(ContractLogger::new(self.context.contract_id.clone()))
            .build()
            .unwrap();

        let elf = self
            .load_contract(self.context.contract_id.clone())
            .context(format!("Load contract {:?}", self.context.contract_id))
            .unwrap();

        info!(contract = ?self.context.contract_id, size = ?elf.len(), "Executing contract");

        let program = risc0_zkvm::Program::load_elf(&elf, MAX_MEMORY).unwrap();
        let image = risc0_zkvm::MemoryImage::new(&program, PAGE_SIZE).unwrap();
        let mut exec = risc0_zkvm::Executor::new(env, image).unwrap();

        let session = exec.run().unwrap();

        ExecutionOutcome::new(session, 0, self.cross_calls_outcomes.take())
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

            let outcome = Executor::new(call_context, self.db.clone()).execute();

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

            let db_key = format!(
                "committed_storage.{}.{}",
                self.context.contract_id.to_string(),
                key
            );

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

            debug!(contract=?self.context.contract_id, key=?key, "Loading storage");

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

            debug!(contract=?self.context.contract_id, key=?request.key, new_hash = utils::bytes_to_hex(hash2.as_slice()), "Updating storage");

            let db_key = format!(
                "committed_storage.{}.{}",
                self.context.contract_id.to_string(),
                request.key
            );

            self.db
                .insert(db_key, request.storage)
                .expect("Failed to insert storage to db");

            Ok(Default::default())
        }
    }
}

struct ContractLogger {
    contract_id: AccountId,
}

impl ContractLogger {
    fn new(contract_id: AccountId) -> Self {
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

// use std::sync::{Arc, RwLock};

// use risc0_zkvm::{serde::to_vec, Executor, ExecutorEnv};
// use multivm_primitives::{
//     syscalls::{CROSS_CONTRACT_CALL, GET_ACCOUNT_MAPPING, GET_STORAGE_CALL, SET_STORAGE_CALL},
//     AccountId, Digest, SignedTransaction, Transaction,
// };

// use crate::syscalls::{
//     accounts_mapping::AccountsMappingHandler, cross_contract::CrossContractCallHandler,
// };
// use crate::{
//     context::ExecutionContext,
//     syscalls::storage::{GetStorageCallHandler, SetStorageCallHandler},
// };

// pub fn bootstrap_tx(db: sled::Db, signed_tx: SignedTransaction) -> Result<risc0_zkvm::Session> {
//     let context = Arc::new(RwLock::new(ExecutionContext::new(
//         multivm_primitives::ContractEntrypointContext {
//             account: signed_tx.transaction.contract.clone(),
//             method: signed_tx.transaction.method.clone(),
//             args: signed_tx.transaction.args.clone(),
//             attached_gas: signed_tx.transaction.attached_gas,
//             sender: signed_tx.transaction.signer.clone(),
//             signer: signed_tx.transaction.signer.clone(),
//         },
//         db,
//     )));

// }

// pub fn execute(context: Arc<RwLock<ExecutionContext>>) -> Result<risc0_zkvm::Session> {
//     let mut exec = {
//         let ctx = context.read().unwrap();
//         debug!(contract = ?ctx.call().account, "Executing contract");

//         let env = ExecutorEnv::builder()
//             .add_input(&to_vec(&ctx.call().into_bytes())?)
//             .session_limit(Some(ctx.call().attached_gas.try_into().unwrap()))
//             .syscall(
//                 CROSS_CONTRACT_CALL,
//                 CrossContractCallHandler::new(context.clone()),
//             )
//             .syscall(
//                 GET_STORAGE_CALL,
//                 GetStorageCallHandler::new(context.clone()),
//             )
//             .syscall(
//                 SET_STORAGE_CALL,
//                 SetStorageCallHandler::new(context.clone()),
//             )
//             .syscall(
//                 GET_ACCOUNT_MAPPING,
//                 AccountsMappingHandler::new(context.clone()),
//             )
//             .stdout(ContractLogger::new(context.clone()))
//             .build()?;

//         let elf = if ctx.call().account == AccountId::new(String::from("evm")) {
//             meta_contracts::EVM_METACONTRACT_ELF.to_vec()
//         } else {
//             load_contract(&ctx.db, ctx.call().account.clone())
//                 .context(format!("Load contract {:?}", ctx.call().account))?
//         };

//         let program = risc0_zkvm::Program::load_elf(&elf, MAX_MEMORY)?;
//         let image = risc0_zkvm::MemoryImage::new(&program, PAGE_SIZE)?;
//         risc0_zkvm::LocalExecutor::new(env, image, program.entry)
//     };

//     let session = exec.run()?;
//     {
//         let cycles = 2u64.pow(
//             session
//                 .segments
//                 .iter()
//                 .map(|s| s.resolve().unwrap().po2)
//                 .sum::<usize>()
//                 .try_into()
//                 .unwrap(),
//         );
//         let mut ctx = context.write().unwrap();
//         ctx.set_gas_usage(cycles);
//     }

//     // debug!("Start proving...");
//     // let _receipt = session.prove();
//     // debug!("Proved");

//     Ok(session)
// }
