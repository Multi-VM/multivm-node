use std::sync::{atomic::AtomicU64, Arc};

use base64::prelude::{Engine as _, BASE64_STANDARD};
use color_eyre::{
    eyre::{eyre, Context, ContextCompat},
    Result,
};
use tokio::task::JoinHandle;
use tracing::{debug, instrument};

use multivm_primitives::{
    AccountId, Block as RawBlock, Event as RawEvent, EvmAddress, MultiVmAccountId,
    Receipt as RawReceipt, SolanaAddress, SupportedTransaction, SystemEvent,
};

use crate::explorer::storage::models::{Account, Block, Event, Receipt, Stats, Transaction};

use super::storage::Storage;

type DbTx<'a> = sqlx::Transaction<'a, sqlx::Sqlite>;

pub struct BlockProcessor {
    storage: Storage,
    tx_counter: Arc<AtomicU64>,
    accounts_counter: Arc<AtomicU64>,
    contract_counter: Arc<AtomicU64>,
}

pub async fn start(
    storage: Storage,
    mut blocks_rx: tokio::sync::broadcast::Receiver<RawBlock>,
) -> JoinHandle<Result<()>> {
    let handle = tokio::spawn(async move {
        loop {
            let block = blocks_rx.recv().await.context("can't receive block")?;
            let db_tx = storage.begin_transaction().await?;
            BlockProcessor::new(storage.clone())
                .process(db_tx, block)
                .await
                .context("can't process block")?;
        }
    });

    handle
}

impl BlockProcessor {
    pub fn new(storage: Storage) -> Self {
        Self {
            storage,
            tx_counter: Arc::new(AtomicU64::new(0)),
            accounts_counter: Arc::new(AtomicU64::new(0)),
            contract_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    #[instrument(skip_all, fields(block_number = raw_block.height))]
    pub async fn process(mut self, mut db_tx: DbTx<'_>, raw_block: RawBlock) -> Result<()> {
        debug!("processing new block");
        let block_ts = raw_block.timestamp;
        let block_id = self.process_block(&mut db_tx, &raw_block).await?;

        for tx in raw_block.txs {
            let receipt = raw_block.receipts.get(&tx.hash()).with_context(|| {
                format!(
                    "receipt not found for transaction {}",
                    hex::encode(tx.hash())
                )
            })?;
            self.process_transaction(&mut db_tx, tx.clone(), receipt.clone(), block_id)
                .await?;
        }

        self.process_stats(&mut db_tx, block_id, block_ts).await?;

        db_tx.commit().await?;

        Ok(())
    }

    #[instrument(skip_all, fields(block_number = raw_block.height))]
    async fn process_block(&mut self, db_tx: &mut DbTx<'_>, raw_block: &RawBlock) -> Result<i64> {
        let block = Block {
            id: 0,
            number: raw_block.height.try_into()?,
            hash: hex::encode(raw_block.hash),
            timestamp: raw_block.timestamp.try_into()?,
            txs_count: raw_block.txs.len().try_into()?,
        };

        Ok(self.storage.insert_block(db_tx, &block).await?)
    }

    async fn process_stats(
        &mut self,
        db_tx: &mut DbTx<'_>,
        block_id: i64,
        block_ts: u64,
    ) -> Result<()> {
        let (total_txs, total_accounts, total_contracts) = self
            .storage
            .find_latest_stats()
            .await?
            .map(|s| (s.total_txs, s.total_accounts, s.total_contracts))
            .unwrap_or_default();

        let tx_counter: i64 = self
            .tx_counter
            .load(std::sync::atomic::Ordering::Relaxed)
            .try_into()?;

        let accounts_counter: i64 = self
            .accounts_counter
            .load(std::sync::atomic::Ordering::Relaxed)
            .try_into()?;

        let contract_counter: i64 = self
            .contract_counter
            .load(std::sync::atomic::Ordering::Relaxed)
            .try_into()?;

        let stats = Stats {
            id: 0,
            timestamp: block_ts.try_into()?,
            block_id,
            total_txs: total_txs + tx_counter,
            total_accounts: total_accounts + accounts_counter,
            total_contracts: total_contracts + contract_counter,
        };

        self.storage.insert_stats(db_tx, &stats).await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn find_account_by_account_id(
        &self,
        db_tx: &mut DbTx<'_>,
        account_id: AccountId,
    ) -> Result<Account> {
        match account_id.clone() {
            AccountId::MultiVm(fvm) => {
                self.storage
                    .dbtx_find_account_by_fvm_address(db_tx, fvm.to_string())
                    .await?
            }
            AccountId::Evm(evm) => {
                self.storage
                    .dbtx_find_account_by_evm_address(db_tx, evm.to_string())
                    .await?
            }
            AccountId::Solana(svm) => {
                self.storage
                    .dbtx_find_account_by_svm_address(db_tx, svm.to_string())
                    .await?
            }
        }
        .ok_or_else(|| eyre!("account not found"))
    }

    #[instrument(skip_all, fields(tx_hash = hex::encode(raw_tx.hash())))]
    async fn process_transaction(
        &self,
        db_tx: &mut DbTx<'_>,
        raw_tx: SupportedTransaction,
        raw_receipt: RawReceipt,
        block_id: i64,
    ) -> Result<()> {
        let signer = raw_tx.signer();
        let reciever = raw_tx.receiver();

        let signer_account = self.find_account_by_account_id(db_tx, signer).await?;
        let receiver_account = self.find_account_by_account_id(db_tx, reciever).await?;

        let format = match raw_tx {
            SupportedTransaction::MultiVm(_) => "FVM",
            SupportedTransaction::Evm(_) => "EVM",
            SupportedTransaction::Solana(_) => "SVM",
        }
        .to_string();

        let tx = Transaction {
            id: 0,
            hash: hex::encode(raw_tx.hash()),
            block_id,
            signer_account_id: signer_account.id,
            receiver_account_id: receiver_account.id,
            format,
            nonce: raw_tx.nonce().try_into()?,
        };

        let tx_id = self.storage.insert_transaction(db_tx, &tx).await?;

        self.process_receipt(db_tx, raw_receipt, tx_id, 0, ReceiptKind::Root, block_id)
            .await?;

        self.tx_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }

    #[async_recursion::async_recursion]
    #[instrument(skip(self, db_tx, raw_receipt))]
    async fn process_receipt(
        &self,
        db_tx: &mut DbTx<'_>,
        raw_receipt: RawReceipt,
        tx_id: i64,
        idx: i64,
        kind: ReceiptKind,
        block_id: i64,
    ) -> Result<i64> {
        let contract_account = self
            .find_account_by_account_id(db_tx, raw_receipt.contract_id)
            .await?;

        let receipt = Receipt {
            id: 0,
            transaction_id: tx_id,
            parent_receipt_id: match kind {
                ReceiptKind::Root => None,
                ReceiptKind::CrossCall { parent_receipt_id } => Some(parent_receipt_id),
            },
            index_in_transaction: idx,
            result: raw_receipt.response.is_ok(),
            response: raw_receipt.response.map(|r| BASE64_STANDARD.encode(r)).ok(),
            gas_used: raw_receipt.gas_used.try_into()?,
            contract_account_id: contract_account.id,
            call_method: raw_receipt.call.method,
            call_args: BASE64_STANDARD.encode(raw_receipt.call.args),
            call_gas: raw_receipt.call.gas.try_into()?,
            call_deposit: raw_receipt.call.deposit.to_string(),
        };

        let parent_receipt_id = self.storage.insert_receipt(db_tx, &receipt).await?;

        let (system_events, contract_events): (Vec<SystemEvent>, Vec<Vec<u8>>) =
            raw_receipt.events.into_iter().fold(
                (Vec::new(), Vec::new()),
                |(mut systems, mut contracts), event| {
                    match event {
                        RawEvent::System(system_event) => systems.extend(Some(system_event)),
                        RawEvent::Contract(contract_event) => {
                            contracts.extend(Some(contract_event))
                        }
                    }
                    (systems, contracts)
                },
            );

        for system_event in system_events {
            self.process_system_event(db_tx, system_event, parent_receipt_id, block_id)
                .await?;
        }

        for (idx, contract_event) in contract_events.into_iter().enumerate() {
            self.process_event(db_tx, contract_event, parent_receipt_id, idx.try_into()?)
                .await?;
        }

        for (idx, cross_call_receipt) in raw_receipt.cross_calls_receipts.into_iter().enumerate() {
            self.process_receipt(
                db_tx,
                cross_call_receipt,
                tx_id,
                idx.try_into()?,
                ReceiptKind::CrossCall { parent_receipt_id },
                block_id,
            )
            .await?;
        }

        Ok(parent_receipt_id)
    }

    #[instrument(skip(self, db_tx, event_data))]
    async fn process_event(
        &self,
        db_tx: &mut DbTx<'_>,
        event_data: Vec<u8>,
        receipt_id: i64,
        idx: i64,
    ) -> Result<i64> {
        let event = Event {
            id: 0,
            receipt_id,
            index_in_receipt: idx,
            message: BASE64_STANDARD.encode(event_data),
        };

        Ok(self.storage.insert_event(db_tx, &event).await?)
    }

    #[instrument(skip(self, db_tx))]
    async fn process_system_event(
        &self,
        db_tx: &mut DbTx<'_>,
        system_event: SystemEvent,
        receipt_id: i64,
        block_id: i64,
    ) -> Result<()> {
        match system_event {
            SystemEvent::AccountCreated(evm, fvm, svm) => {
                self.process_account_creation(db_tx, fvm, evm, svm, block_id)
                    .await?;
            }
            SystemEvent::ContractDeployed(account_id) => {
                self.process_contract_deployment(db_tx, account_id, block_id)
                    .await?;
            }
            SystemEvent::BalanceChanged(account_id, new_balance) => {
                self.process_balance_update(db_tx, account_id, new_balance, block_id)
                    .await?;
            }
        }

        Ok(())
    }

    #[instrument(skip(self, db_tx, block_id))]
    async fn process_account_creation(
        &self,
        db_tx: &mut DbTx<'_>,
        fvm: Option<MultiVmAccountId>,
        evm: EvmAddress,
        svm: SolanaAddress,
        block_id: i64,
    ) -> Result<i64> {
        let account = Account {
            id: 0,
            fvm_address: fvm.map(|fvm| fvm.to_string()),
            evm_address: evm.to_string(),
            svm_address: svm.to_string(),
            created_at_block_id: block_id,
            modified_at_block_id: block_id,
            executable_type: None,
            native_balance: 1000000000000000000000u128.to_string(), // Value from Meta Contract. TODO: add this value to event
        };

        let account_id = self.storage.insert_account(db_tx, &account).await?;

        self.accounts_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(account_id)
    }

    #[instrument(skip(self, db_tx, block_id))]
    async fn process_balance_update(
        &self,
        db_tx: &mut DbTx<'_>,
        account_id: AccountId,
        new_balance: u128,
        block_id: i64,
    ) -> Result<()> {
        let account = self.find_account_by_account_id(db_tx, account_id).await?;

        self.storage
            .update_account_balance(db_tx, account.id, new_balance.to_string(), block_id)
            .await?;

        Ok(())
    }

    #[instrument(skip(self, db_tx, block_id))]
    async fn process_contract_deployment(
        &self,
        db_tx: &mut DbTx<'_>,
        account_id: AccountId,
        block_id: i64,
    ) -> Result<()> {
        let executable_type = match account_id {
            AccountId::MultiVm(_) => "FVM",
            AccountId::Evm(_) => "EVM",
            AccountId::Solana(_) => "SVM",
        }
        .to_string();

        let account = self.find_account_by_account_id(db_tx, account_id).await?;

        self.storage
            .update_account_executable_type(db_tx, account.id, Some(executable_type), block_id)
            .await?;

        self.contract_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }
}

#[derive(Debug)]
enum ReceiptKind {
    Root,
    CrossCall { parent_receipt_id: i64 },
}
