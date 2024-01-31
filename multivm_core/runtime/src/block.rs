use std::collections::HashMap;

use multivm_primitives::{Block, Digest, SupportedTransaction};
use tracing::info;

use crate::outcome::ExecutionOutcome;

pub struct UnprovedBlock {
    pub height: u64,
    pub hash: Digest,
    pub parent_hash: Digest,
    pub previous_global_root: Digest,
    pub new_global_root: Digest,
    pub timestamp: u64,
    pub txs: Vec<SupportedTransaction>,
    pub execution_outcomes: HashMap<Digest, ExecutionOutcome>,
}

impl UnprovedBlock {
    // TODO: prove block with zk
    pub fn prove(self, skip_proof: bool) -> Block {
        let receipts: HashMap<_, _> = self
            .execution_outcomes
            .iter()
            .map(|(hash, outcome)| {
                if !skip_proof 
                {
                        let methods = match self
                            .txs
                            .iter()
                            .find(|tx| tx.hash() == *hash)
                            .unwrap() {
                                SupportedTransaction::MultiVm(multivm_tx) => {
                                    multivm_tx.transaction.calls.iter().map(|call| call.method.clone()).collect::<Vec<_>>()
                                }
                                // TODO: replace with proper method
                                SupportedTransaction::Evm(_) => {vec!["evm call".to_string()]}
                                SupportedTransaction::Solana(_) => {vec!["solana call".to_string()]}
                            };

                        let start = std::time::Instant::now();

                        info!(tx_hash = ?eth_primitive_types::H256::from(hash), methods = ?methods, "Proving outcome...");
                        let _outcome = outcome.prove_all();
                        info!(
                            tx_hash = ?eth_primitive_types::H256::from(hash),
                            methods = ?methods,
                            duration = ?start.elapsed(),
                            "Outcome proved",
                        );
                };
                
                let receipt = outcome.receipts();

                (hash.clone(), receipt)
            })
            .collect();

        let call_outputs = receipts
            .iter()
            .map(|(hash, commitment)| (hash.clone(), commitment.response.clone()))
            .collect();

        Block {
            height: self.height,
            hash: self.hash,
            parent_hash: self.parent_hash,
            previous_global_root: self.previous_global_root,
            new_global_root: self.new_global_root,
            timestamp: self.timestamp,
            txs: self.txs,
            call_outputs,
            receipts,
        }
    }
}
