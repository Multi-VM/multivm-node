use anyhow::{Context, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use tracing::{debug, span, Level};

use multivm_primitives::{
    syscalls::{GetStorageResponse, GET_STORAGE_CALL},
    AccountId, Commitment, ContractCallContext, SignedTransaction, SYSTEM_META_CONTRACT_ACCOUNT_ID,
};

const MAX_MEMORY: u32 = 0x10000000;
const PAGE_SIZE: u32 = 0x400;

#[derive(BorshDeserialize, BorshSerialize)]
enum Action {
    ExecuteTransaction(SignedTransaction),
    View(ContractCallContext),
}

pub struct Viewer {
    context: ContractCallContext,
    db: sled::Db,
}

impl Viewer {
    pub fn new(context: ContractCallContext, db: sled::Db) -> Self {
        Self { context, db }
    }

    pub fn view(self) -> Vec<u8> {
        let action = Action::View(self.context.clone());
        let action_bytes = borsh::to_vec(&action).unwrap();

        let env = risc0_zkvm::ExecutorEnv::builder()
            .add_input(&risc0_zkvm::serde::to_vec(&action_bytes).unwrap())
            .session_limit(Some(usize::MAX))
            .io_callback(GET_STORAGE_CALL, self.callback_on_get_storage())
            .stdout(ContractLogger::new(self.context.contract_id.clone()))
            .build()
            .unwrap();

        let elf = if self.context.contract_id.to_string().as_str() == "evm" {
            meta_contracts::EVM_METACONTRACT_ELF.to_vec()
        } else if self.context.contract_id.to_string().as_str() == "multivm" {
            meta_contracts::ROOT_METACONTRACT_ELF.to_vec()
        } else {
            self.load_contract(self.context.contract_id.clone())
                .context(format!("Load contract {:?}", self.context.contract_id))
                .unwrap()
        };

        let program = risc0_zkvm::Program::load_elf(&elf, MAX_MEMORY).unwrap();
        let image = risc0_zkvm::MemoryImage::new(&program, PAGE_SIZE).unwrap();
        let mut exec = risc0_zkvm::Executor::new(env, image).unwrap();

        let session = exec.run().unwrap();

        Commitment::try_from_bytes(session.journal.clone())
            .expect("Corrupted journal")
            .response
    }

    fn load_contract(&self, contract_id: AccountId) -> Result<Vec<u8>> {
        let db_key = format!(
            "committed_storage.{}.code.{}",
            SYSTEM_META_CONTRACT_ACCOUNT_ID,
            contract_id.to_string(),
        );

        let code = self
            .db
            .get(db_key)
            .expect("Failed to get storage from db")
            .map(|v| v.to_vec())
            .expect("Contract not found");

        let code = BorshDeserialize::deserialize(&mut code.as_slice()).unwrap();

        Ok(code)
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

        tracing::debug!(contract_id = ?self.contract_id, msg, "📜 Contract log");

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        unimplemented!()
    }
}
