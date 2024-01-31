use color_eyre::Result;
use sqlx::{query_as, Transaction};
use tracing::instrument;

use crate::explorer::storage::{models::receipts::Receipt, Storage};

impl Storage {
    #[instrument(skip(self))]
    pub async fn find_receipt_by_id(&self, id: i64) -> Result<Option<Receipt>> {
        Ok(query_as!(
            Receipt,
            r#"SELECT *
            FROM receipts
            WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    #[instrument(skip(self))]
    pub async fn find_receipts_by_transaction_id(
        &self,
        transaction_id: i64,
    ) -> Result<Vec<Receipt>> {
        Ok(query_as!(
            Receipt,
            r#"SELECT *
            FROM receipts
            WHERE transaction_id = ?"#,
            transaction_id
        )
        .fetch_all(&self.pool)
        .await?)
    }

    #[instrument(skip(self))]
    pub async fn find_root_receipt(&self, transaction_id: i64) -> Result<Option<Receipt>> {
        Ok(query_as!(
            Receipt,
            r#"SELECT *
            FROM receipts
            WHERE transaction_id = ? AND parent_receipt_id is NULL"#,
            transaction_id
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    #[instrument(skip(self))]
    pub async fn find_cross_calls_receipt(&self, parent_receipt_id: i64) -> Result<Vec<Receipt>> {
        let mut receipts = query_as!(
            Receipt,
            r#"SELECT *
            FROM receipts
            WHERE parent_receipt_id = ?"#,
            parent_receipt_id
        )
        .fetch_all(&self.pool)
        .await?;
        receipts.sort_by(|a, b| a.index_in_transaction.cmp(&b.index_in_transaction));
        Ok(receipts)
    }

    #[instrument(skip(self))]
    pub async fn insert_receipt(
        &self,
        db_tx: &mut Transaction<'_, sqlx::Sqlite>,
        receipt: &Receipt,
    ) -> Result<i64> {
        Ok(sqlx::query!(
            r#"INSERT INTO receipts (
                transaction_id,
                parent_receipt_id,
                index_in_transaction,
                result,
                response,
                gas_used,
                contract_account_id,
                call_method,
                call_args,
                call_gas,
                call_deposit
            ) VALUES (?,?,?,?,?,?,?,?,?,?,?)
            RETURNING id
            "#,
            receipt.transaction_id,
            receipt.parent_receipt_id,
            receipt.index_in_transaction,
            receipt.result,
            receipt.response,
            receipt.gas_used,
            receipt.contract_account_id,
            receipt.call_method,
            receipt.call_args,
            receipt.call_gas,
            receipt.call_deposit,
        )
        .fetch_one(db_tx.as_mut())
        .await?
        .id)
    }
}
