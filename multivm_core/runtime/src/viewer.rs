use borsh::{BorshDeserialize, BorshSerialize};
use color_eyre::{
    eyre::{bail, eyre},
    Result,
};
use tracing::{debug, instrument, span, Level};

use multivm_primitives::{
    syscalls::{GetStorageResponse, GET_STORAGE_CALL, SET_STORAGE_CALL},
    AccountId, Commitment, ContractCall, ContractCallContext, ContractResponse, EnvironmentContext,
    EvmAddress, MultiVmAccountId, SupportedTransaction,
};

use crate::account::{Account, Executable};

const MAX_MEMORY: u32 = 0x10000000;
const PAGE_SIZE: u32 = 0x400;

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub struct EvmCall {
    pub from: Option<[u8; 20]>,
    pub to: [u8; 20],
    pub input: Vec<u8>,
}

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub enum SupportedView {
    MultiVm(ContractCallContext),
    Evm(EvmCall),
}

impl SupportedView {
    pub fn contract_id(&self) -> AccountId {
        match self {
            SupportedView::MultiVm(context) => context.contract_id.clone(),
            SupportedView::Evm(call) => EvmAddress::from(call.to.clone()).into(),
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
enum Action {
    ExecuteTransaction(SupportedTransaction, EnvironmentContext),
    View(SupportedView, EnvironmentContext),
}

pub struct Viewer {
    view: SupportedView,
    db: sled::Db,
}

impl Viewer {
    pub fn new(view: SupportedView, db: sled::Db) -> Self {
        Self { view, db }
    }

    #[instrument(skip(db))]
    pub fn account_info(account_id: &AccountId, db: sled::Db) -> Result<Option<Account>> {
        let bytes = Viewer::view_system_meta_contract("account_info".to_string(), account_id, db)?
            .map_err(|err| eyre!("meta contract failed: {:?}", err))?;

        Ok(borsh::from_slice(&bytes)?)
    }

    #[instrument(skip(args, db), fields(method=?method))]
    pub fn view_system_meta_contract<T: BorshSerialize>(
        method: String,
        args: &T,
        db: sled::Db,
    ) -> Result<ContractResponse> {
        let context = ContractCallContext {
            contract_id: AccountId::system_meta_contract(),
            contract_call: ContractCall::new(method, args, 100_000_000, 0),
            sender_id: AccountId::system_meta_contract(),
            signer_id: AccountId::system_meta_contract(),
            environment: EnvironmentContext { block_height: 0 }, // TODO: hardcoded height
        };

        let action = Action::View(SupportedView::MultiVm(context.clone()), context.environment);
        let input_bytes = borsh::to_vec(&action)?;

        let env = risc0_zkvm::ExecutorEnv::builder()
            .write_slice(&input_bytes)
            .session_limit(Some(u64::MAX))
            .io_callback(GET_STORAGE_CALL, callback_on_system_get_storage(db.clone()))
            .stdout(ContractLogger::new(AccountId::system_meta_contract()))
            .build()
            .map_err(|err| eyre!(Box::new(err)))?;

        let program = risc0_zkvm::Program::load_elf(
            &meta_contracts::SYSTEM_META_CONTRACT_ELF.to_vec(),
            MAX_MEMORY,
        )
        .unwrap();
        let image = risc0_zkvm::MemoryImage::new(&program, PAGE_SIZE).unwrap();
        let exec = risc0_zkvm::default_executor();

        let session = exec.execute(env, image).unwrap();

        Ok(Commitment::try_from_bytes(session.journal.bytes.clone())
            .map_err(|err| eyre!(Box::new(err)))?
            .response)
    }

    #[instrument(skip(self), fields(contract_id=?self.view.contract_id()))]
    pub fn view(self) -> Result<ContractResponse> {
        let contract_id = self.view.contract_id();

        debug!(contract_id=?contract_id, "Viewing contract");

        let (input_bytes, elf) = if contract_id != AccountId::system_meta_contract() {
            let contract = Viewer::account_info(&contract_id, self.db.clone())?
                .ok_or_else(|| eyre!("contract not found in the database"))?;

            match contract.executable {
                Some(Executable::MultiVm(_)) | Some(Executable::Solana(_)) => {
                    match self.view.clone() {
                        SupportedView::MultiVm(view) => {
                            let input_bytes = borsh::to_vec(&view)?;
                            (
                                input_bytes,
                                self.load_contract(&contract.multivm_account_id.unwrap())?,
                            )
                        }
                        _ => bail!("Non-MultiVM view for MultiVM contract"),
                    }
                }
                Some(Executable::Evm()) => {
                    let action = borsh::to_vec(&Action::View(
                        self.view.clone(),
                        EnvironmentContext { block_height: 0 },
                    ))?;
                    (action, meta_contracts::SYSTEM_META_CONTRACT_ELF.to_vec())
                }
                None => bail!("Viewing non-executable account"),
            }
        } else {
            (
                borsh::to_vec(&Action::View(
                    self.view.clone(),
                    EnvironmentContext { block_height: 0 },
                ))?,
                meta_contracts::SYSTEM_META_CONTRACT_ELF.to_vec(),
            )
        };

        let env = risc0_zkvm::ExecutorEnv::builder()
            .write_slice(&input_bytes)
            .session_limit(Some(u64::MAX))
            .io_callback(GET_STORAGE_CALL, self.callback_on_get_storage())
            .io_callback(SET_STORAGE_CALL, self.callback_on_set_storage())
            .stdout(ContractLogger::new(AccountId::system_meta_contract()))
            .build()
            .map_err(|err| eyre!(Box::new(err)))?;

        let program =
            risc0_zkvm::Program::load_elf(&elf, MAX_MEMORY).map_err(|err| eyre!(Box::new(err)))?;
        let image = risc0_zkvm::MemoryImage::new(&program, PAGE_SIZE)
            .map_err(|err| eyre!(Box::new(err)))?;
        let exec = risc0_zkvm::default_executor();

        let session = exec
            .execute(env, image)
            .map_err(|err| eyre!(Box::new(err)))?;

        Ok(Commitment::try_from_bytes(session.journal.bytes.clone())?.response)
    }

    fn load_contract(&self, contract_id: &MultiVmAccountId) -> Result<Vec<u8>> {
        let db_key = format!("contracts_code.{}", contract_id.to_string());

        let code = self
            .db
            .get(db_key)?
            .map(|v| v.to_vec())
            .ok_or_else(|| eyre!("contract not found"))?;

        Ok(code)
    }

    pub fn callback_on_get_storage<'a>(
        &'a self,
    ) -> impl Fn(risc0_zkvm::Bytes) -> risc0_zkvm::Result<risc0_zkvm::Bytes> + 'a {
        |from_guest| {
            let span = span!(Level::DEBUG, "get_storage call handler");
            let _enter = span.enter();

            let key = String::from_utf8(from_guest.into()).unwrap();

            let storage_location = if self.view.contract_id() != AccountId::system_meta_contract() {
                let contract = Viewer::account_info(&self.view.contract_id(), self.db.clone())
                    .unwrap()
                    .expect("Loading storage for non-existent contract");

                let storage_location = match contract.executable {
                    Some(Executable::MultiVm(_)) | Some(Executable::Solana(_)) => contract
                        .multivm_account_id
                        .expect("Contract without MultiVmAccountId")
                        .into(),
                    Some(Executable::Evm()) => AccountId::system_meta_contract(),
                    None => unreachable!("Loading storage for non-executable account"),
                };
                storage_location
            } else {
                AccountId::system_meta_contract()
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

            debug!(contract=?self.view.contract_id(), key=?key, "Loading storage");

            Ok(response_bytes.into())
        }
    }

    // Writing to storage is prohibited in viewer
    pub fn callback_on_set_storage<'a>(
        &'a self,
    ) -> impl Fn(risc0_zkvm::Bytes) -> risc0_zkvm::Result<risc0_zkvm::Bytes> + 'a {
        |_from_guest| Ok(Default::default())
    }
}

fn callback_on_system_get_storage(
    db: sled::Db,
) -> impl Fn(risc0_zkvm::Bytes) -> risc0_zkvm::Result<risc0_zkvm::Bytes> {
    move |from_guest| {
        let span = span!(Level::DEBUG, "get_storage call handler");
        let _enter = span.enter();

        let key = String::from_utf8(from_guest.into()).unwrap();

        let db_key = format!(
            "committed_storage.{}.{}",
            AccountId::system_meta_contract(),
            key
        );

        let storage = db
            .get(db_key)
            .expect("Failed to get storage from db")
            .map(|v| v.to_vec());

        let response = GetStorageResponse { storage };

        let response_bytes = borsh::to_vec(&response).unwrap();

        debug!(contract=?AccountId::system_meta_contract(), key=?key, "Loading system storage");

        Ok(response_bytes.into())
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
