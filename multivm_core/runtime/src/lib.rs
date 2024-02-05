use std::{collections::HashMap, time::UNIX_EPOCH};

use account::{Account, Executable};
use block::UnprovedBlock;
use bootstraper::Bootstraper;
use borsh::BorshSerialize;
use color_eyre::{eyre::eyre, Result};
use multivm_primitives::{
    AccountId, Block, ContractResponse, EnvironmentContext, SupportedTransaction,
};
use tracing::{debug, info, instrument};
use viewer::{SupportedView, Viewer};

pub mod account;
pub mod block;
pub mod bootstraper;
pub mod executor;
pub mod outcome;
pub mod utils;
pub mod viewer;

pub struct MultivmNode {
    db: sled::Db,
    txs_pool: std::collections::VecDeque<SupportedTransaction>,
    events_tx: tokio::sync::broadcast::Sender<Block>,
}

impl MultivmNode {
    #[instrument(skip(events_tx))]
    pub fn new(db_path: String, events_tx: tokio::sync::broadcast::Sender<Block>) -> Result<Self> {
        info!(db_path, "Starting node");

        let mut node = Self {
            db: sled::open(db_path)?,
            txs_pool: std::collections::VecDeque::new(),
            events_tx,
        };

        if !node.db.was_recovered() {
            node.init_genesis()?;
        }

        Ok(node)
    }

    #[instrument(skip(self))]
    pub fn init_genesis(&mut self) -> Result<()> {
        info!("Initializing genesis block");
        let genesis_block = Block {
            height: 0,
            hash: [0; 32],
            parent_hash: [0; 32],
            previous_global_root: Default::default(),
            new_global_root: Default::default(),
            timestamp: std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            txs: Default::default(),
            call_outputs: Default::default(),
            receipts: Default::default(),
        };

        self.insert_block(genesis_block)?;

        Ok(())
    }

    #[instrument(skip(self))]
    fn insert_block(&mut self, block: Block) -> Result<()> {
        self.db
            .insert(format!("block_{}", block.height), borsh::to_vec(&block)?)?;

        self.db.insert(b"latest_block", borsh::to_vec(&block)?)?;

        self.db.flush()?;

        self.events_tx.send(block)?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn block_by_height(&self, height: u64) -> Result<Option<Block>> {
        let block = self
            .db
            .get(format!("block_{}", height))?
            .map(|bytes| borsh::from_slice(&mut bytes.to_vec()))
            .transpose()?;

        Ok(block)
    }

    #[instrument(skip(self))]
    pub fn latest_block(&self) -> Result<Block> {
        let mut latest_block_bytes = self
            .db
            .get(b"latest_block")?
            .ok_or_else(|| eyre!("latest block not found"))?
            .to_vec();
        let latest_block: Block = borsh::from_slice(&mut latest_block_bytes)?;
        debug!(height = latest_block.height, "Latest block in runtime");

        Ok(latest_block)
    }

    pub fn add_tx(&mut self, tx: SupportedTransaction) {
        self.txs_pool.push_back(tx);
    }

    #[instrument(skip(self))]
    fn environment(&self) -> Result<EnvironmentContext> {
        Ok(EnvironmentContext {
            block_height: self.latest_block()?.height + 1,
        })
    }

    pub fn txs_count(&self) -> usize {
        self.txs_pool.len()
    }

    #[instrument(skip(self))]
    pub fn produce_block(&mut self, skip_proof: bool) -> Result<Block> {
        let latest_block = self.latest_block()?;
        info!(height = latest_block.height + 1, "Creating new block");
        let start: std::time::Instant = std::time::Instant::now();
        // self.txs_pool = Default::default();
        let env = self.environment()?;
        let (txs, execution_outcomes): (Vec<_>, Vec<_>) = self
            .txs_pool
            .iter()
            .map(|tx| {
                let outcome =
                    Bootstraper::new(self.db.clone(), tx.clone(), tx.signer(), env.clone())
                        .bootstrap();
                (tx.clone(), outcome)
            })
            .unzip();

        let execution_outcomes: HashMap<_, _> = txs
            .iter()
            .map(|tx| tx.hash())
            .zip(execution_outcomes)
            .collect();

        self.txs_pool = Default::default();

        let hash = {
            let mut hash: [u8; 32] = [0; 32];
            let (one, two) = hash.split_at_mut(24);
            one.copy_from_slice(&[0; 24]);
            two.copy_from_slice(&(latest_block.height + 1).to_be_bytes());
            hash
        };

        let unproved_block = UnprovedBlock {
            height: latest_block.height + 1,
            hash,
            parent_hash: latest_block.hash,
            timestamp: std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            txs,
            previous_global_root: Default::default(),
            new_global_root: Default::default(),
            execution_outcomes,
        };

        let block = unproved_block.prove(skip_proof);
        info!(time = ?start.elapsed(), height = block.height, txs_count = block.txs.len(), "Block created");
        debug!(height = ?block.height, txs = ?block.txs.iter().map(|tx| hex::encode(tx.hash())).collect::<Vec<_>>(), "Block created");
        self.insert_block(block.clone())?;

        Ok(block)
    }

    #[instrument(skip(self))]
    pub fn account_info(&self, account_id: &AccountId) -> Result<Option<Account>> {
        Ok(Viewer::account_info(account_id, self.db.clone())?)
    }

    #[instrument(skip(self, args))]
    pub fn system_view<T: BorshSerialize>(
        &self,
        method: String,
        args: &T,
    ) -> Result<ContractResponse> {
        Viewer::view_system_meta_contract(method, args, self.db.clone())
    }

    #[instrument(skip(self))]
    pub fn contract_view(&self, view: SupportedView) -> Result<ContractResponse> {
        Viewer::new(view, self.db.clone()).view()
    }

    #[instrument(skip(self))]
    pub fn account_raw_storage(
        &self,
        account_id: AccountId,
        key: String,
    ) -> Result<Option<Vec<u8>>> {
        let storage_location = if account_id == AccountId::system_meta_contract() {
            AccountId::system_meta_contract()
        } else {
            let contract = Viewer::account_info(&account_id, self.db.clone())?
                .ok_or_else(|| eyre!("loading storage for non-existent contract"))?;

            match contract.executable {
                Some(Executable::MultiVm(_)) | Some(Executable::Solana(_)) => contract
                    .multivm_account_id
                    .expect("Contract without MultiVmAccountId")
                    .into(),
                Some(Executable::Evm()) => AccountId::system_meta_contract(),
                None => unreachable!("Loading storage for non-executable account"),
            }
        };

        let db_key = format!("committed_storage.{}.{}", storage_location, key);

        let storage = self.db.get(db_key)?.map(|v| v.to_vec());

        Ok(storage)
    }
}
