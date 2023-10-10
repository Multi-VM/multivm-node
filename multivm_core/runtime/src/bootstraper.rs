use borsh::{BorshDeserialize, BorshSerialize};
use risc0_zkvm::sha::{Impl as HashImpl, Sha256};
use tracing::{debug, span, Level};

use multivm_primitives::{
    syscalls::{
        CrossContractCallRequest, DeployContractRequest, GetStorageResponse, SetStorageRequest,
        CROSS_CONTRACT_CALL, DEPLOY_CONTRACT_CALL, GET_STORAGE_CALL, SET_STORAGE_CALL,
    },
    AccountId, Attachments, ContractCallContext, EnvironmentContext, SupportedTransaction,
};

use crate::{executor::Executor, outcome::ExecutionOutcome, utils};

use std::{cell::RefCell, rc::Rc};

const MAX_MEMORY: u32 = 0x10000000;
const PAGE_SIZE: u32 = 0x400;

#[derive(BorshDeserialize, BorshSerialize)]
enum Action {
    ExecuteTransaction(SupportedTransaction, EnvironmentContext),
    View(ContractCallContext),
}

pub struct Bootstraper {
    db: sled::Db,
    transaction: SupportedTransaction,
    attachments: Option<Attachments>,
    cross_calls_outcomes: Rc<RefCell<Vec<ExecutionOutcome>>>,
    environment: EnvironmentContext,
}

impl Bootstraper {
    pub fn new(
        db: sled::Db,
        transaction: SupportedTransaction,
        environment: EnvironmentContext,
    ) -> Self {
        let attachments = match &transaction {
            SupportedTransaction::MultiVm(multivm_tx) => Some(multivm_tx.attachments.clone()),
            SupportedTransaction::Evm(_) => None,
        }
        .flatten();

        Self {
            db,
            transaction,
            attachments,
            cross_calls_outcomes: Default::default(),
            environment,
        }
    }

    pub fn bootstrap(self) -> ExecutionOutcome {
        debug!(
            // tx_hash = utils::bytes_to_hex(self.signed_tx.transaction.hash().as_slice()),
            "Bootstraping transaction"
        );

        let action = Action::ExecuteTransaction(self.transaction.clone(), self.environment.clone());

        let action_bytes = borsh::to_vec(&action).unwrap();

        let env = risc0_zkvm::ExecutorEnv::builder()
            .add_input(&risc0_zkvm::serde::to_vec(&action_bytes).unwrap())
            .session_limit(Some(usize::MAX))
            .io_callback(CROSS_CONTRACT_CALL, self.callback_on_cross_contract_call())
            .io_callback(GET_STORAGE_CALL, self.callback_on_get_storage())
            .io_callback(SET_STORAGE_CALL, self.callback_on_set_storage())
            .io_callback(DEPLOY_CONTRACT_CALL, self.callback_on_contract_deployment())
            .build()
            .unwrap();

        let elf = meta_contracts::SYSTEM_META_CONTRACT_ELF.to_vec();

        let program = risc0_zkvm::Program::load_elf(&elf, MAX_MEMORY).unwrap();
        let image = risc0_zkvm::MemoryImage::new(&program, PAGE_SIZE).unwrap();
        let mut exec = risc0_zkvm::Executor::new(env, image).unwrap();

        let session = exec.run().unwrap();

        ExecutionOutcome::new(session, 0, self.cross_calls_outcomes.take())
    }

    pub fn callback_on_contract_deployment<'a>(
        &'a self,
    ) -> impl Fn(risc0_zkvm::Bytes) -> risc0_zkvm::Result<risc0_zkvm::Bytes> + 'a {
        |from_guest| {
            let span = span!(Level::DEBUG, "contract_deployment call handler");
            let _enter = span.enter();

            let request: DeployContractRequest =
                BorshDeserialize::try_from_slice(&from_guest).unwrap();

            let image_bytes = self
                .attachments
                .as_ref()
                .map(|attachments| attachments.contracts_images.get(&request.image_id))
                .flatten();

            // TODO: error handling
            let Some(image_bytes) = image_bytes else {
                panic!("Contract image not found");
            };

            let program = risc0_zkvm::Program::load_elf(&image_bytes, MAX_MEMORY).unwrap();
            let image = risc0_zkvm::MemoryImage::new(&program, PAGE_SIZE).unwrap();

            // TODO: error handling
            if image.compute_id() != request.image_id.into() {
                panic!("Invalid image id");
            }

            let db_key = format!("contracts_code.{}", request.account_id.to_string());

            self.db
                .insert(db_key, image_bytes.clone())
                .expect("Failed to insert image to db");

            Ok(Default::default())
        }
    }

    pub fn callback_on_cross_contract_call<'a>(
        &'a self,
    ) -> impl Fn(risc0_zkvm::Bytes) -> risc0_zkvm::Result<risc0_zkvm::Bytes> + 'a {
        |from_guest| {
            debug!("Handling syscall for contract call from meta contract");

            let req: CrossContractCallRequest = BorshDeserialize::try_from_slice(&from_guest)
                .expect("Invalid contract call request");

            // TODO: fix
            let SupportedTransaction::MultiVm(multivm_tx) = self.transaction.clone() else {
                panic!("Invalid transaction type");
            };

            let call_context = ContractCallContext {
                contract_id: req.contract_id,
                contract_call: req.contract_call,
                sender_id: multivm_tx.transaction.signer_id.clone(),
                signer_id: multivm_tx.transaction.signer_id.clone(),
                environment: self.environment.clone(),
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
                AccountId::system_meta_contract(),
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

            debug!(contract=?AccountId::system_meta_contract(), key=?key, "Loading storage");

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

            let hash2 = HashImpl::hash_bytes(&request.storage);
            // assert_eq!(request.hash, hash2.as_slice());

            debug!(contract=?AccountId::system_meta_contract(), key=?request.key, new_hash = utils::bytes_to_hex(hash2.as_bytes()), "Updating storage");

            let db_key = format!(
                "committed_storage.{}.{}",
                AccountId::system_meta_contract(),
                request.key
            );

            self.db
                .insert(db_key, request.storage)
                .expect("Failed to insert storage to db");

            Ok(Default::default())
        }
    }
}
