use std::collections::HashMap;

use account::{Account, Executable};
use block::UnprovedBlock;
use bootstraper::Bootstraper;
use borsh::BorshSerialize;
use multivm_primitives::{
    AccountId, Block, ContractResponse, EnvironmentContext, SupportedTransaction,
};
use tracing::{debug, info};
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
}

impl MultivmNode {
    pub fn new(db_path: String) -> Self {
        info!(db_path, "Starting node");

        let mut node = Self {
            db: sled::open(db_path).unwrap(),
            txs_pool: std::collections::VecDeque::new(),
        };

        if !node.db.was_recovered() {
            node.init_genesis();
        }

        node
    }

    pub fn init_genesis(&mut self) {
        info!("Initializing genesis block");
        let genesis_block = Block {
            height: 0,
            hash: [0; 32],
            parent_hash: [0; 32],
            previous_global_root: Default::default(),
            new_global_root: Default::default(),
            timestamp: 0,
            txs: Default::default(),
            call_outputs: Default::default(),
        };

        self.insert_block(genesis_block);
    }

    fn insert_block(&mut self, block: Block) {
        self.db
            .insert(
                format!("block_{}", block.height),
                borsh::to_vec(&block).unwrap(),
            )
            .unwrap();

        self.db
            .insert(b"latest_block", borsh::to_vec(&block).unwrap())
            .unwrap();

        self.db.flush().unwrap();
    }

    pub fn block_by_height(&self, height: u64) -> Option<Block> {
        let block = self
            .db
            .get(format!("block_{}", height))
            .unwrap()
            .map(|bytes| borsh::from_slice(&mut bytes.to_vec()).unwrap());

        block
    }

    pub fn latest_block(&self) -> Block {
        let mut latest_block_bytes = self.db.get(b"latest_block").unwrap().unwrap().to_vec();
        let latest_block: Block = borsh::from_slice(&mut latest_block_bytes).unwrap();

        latest_block
    }

    pub fn add_tx(&mut self, tx: SupportedTransaction) {
        self.txs_pool.push_back(tx);
    }

    fn environment(&self) -> EnvironmentContext {
        EnvironmentContext {
            block_height: self.latest_block().height + 1,
        }
    }

    pub fn produce_block(&mut self, skip_proof: bool) -> Block {
        let latest_block = self.latest_block();
        info!(height = latest_block.height + 1, "Creating new block");
        let start: std::time::Instant = std::time::Instant::now();
        // self.txs_pool = Default::default();
        let (txs, execution_outcomes): (Vec<_>, Vec<_>) = self
            .txs_pool
            .iter()
            .map(|tx| {
                let outcome =
                    Bootstraper::new(self.db.clone(), tx.clone(), tx.signer(), self.environment())
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
            timestamp: 0,
            txs,
            previous_global_root: Default::default(),
            new_global_root: Default::default(),
            execution_outcomes,
        };

        let block = unproved_block.prove(skip_proof);
        info!(time = ?start.elapsed(), height = block.height, txs_count = block.txs.len(), "Block created");
        debug!(height = ?block.height, txs = ?block.txs.iter().map(|tx| hex::encode(tx.hash())).collect::<Vec<_>>(), "Block created");
        self.insert_block(block.clone());
        block
    }

    pub fn account_info(&self, account_id: &AccountId) -> Option<Account> {
        Viewer::account_info(account_id, self.db.clone())
    }

    pub fn system_view<T: BorshSerialize>(&self, method: String, args: &T) -> ContractResponse {
        Viewer::view_system_meta_contract(method, args, self.db.clone())
    }

    pub fn contract_view(&self, view: SupportedView) -> ContractResponse {
        Viewer::new(view, self.db.clone()).view()
    }

    pub fn account_raw_storage(&self, account_id: AccountId, key: String) -> Option<Vec<u8>> {
        let storage_location = if account_id == AccountId::system_meta_contract() {
            AccountId::system_meta_contract()
        } else {
            let contract = Viewer::account_info(&account_id, self.db.clone())
                .expect("Loading storage for non-existent contract");

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

        let storage = self
            .db
            .get(db_key)
            .expect("Failed to get storage from db")
            .map(|v| v.to_vec());

        storage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_genesis() {
        let mut node = MultivmNode::new("temp_multivm_db".to_string());
        node.init_genesis();

        let latest_block = node.latest_block();
        assert_eq!(latest_block.height, 1);
    }
}
