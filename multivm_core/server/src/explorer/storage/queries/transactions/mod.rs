use color_eyre::Result;
use sqlx::{query_as, Transaction as DbTx};
use tracing::instrument;

use crate::explorer::storage::{models::transactions::Transaction, Storage};

impl Storage {
    #[instrument(skip(self))]
    pub async fn find_transaction_by_hash(&self, hash: String) -> Result<Option<Transaction>> {
        Ok(query_as!(
            Transaction,
            r#"SELECT *
            FROM transactions
            WHERE hash = ?"#,
            hash
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    #[instrument(skip(self))]
    pub async fn find_transaction_by_id(&self, id: i64) -> Result<Option<Transaction>> {
        Ok(query_as!(
            Transaction,
            r#"SELECT *
            FROM transactions
            WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    #[instrument(skip(self))]
    pub async fn find_transactions_by_block_id(&self, block_id: i64) -> Result<Vec<Transaction>> {
        Ok(query_as!(
            Transaction,
            r#"SELECT *
            FROM transactions
            WHERE block_id = ?"#,
            block_id
        )
        .fetch_all(&self.pool)
        .await?)
    }

    #[instrument(skip(self, db_tx))]
    pub async fn insert_transaction(
        &self,
        db_tx: &mut DbTx<'_, sqlx::Sqlite>,
        tx: &Transaction,
    ) -> Result<i64> {
        Ok(sqlx::query!(
            r#"INSERT INTO transactions (hash, block_id, signer_account_id, receiver_account_id, format, nonce)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
            tx.hash,
            tx.block_id,
            tx.signer_account_id,
            tx.receiver_account_id,
            tx.format,
            tx.nonce
        )
        .fetch_one(db_tx.as_mut())
        .await?.id)
    }
}
