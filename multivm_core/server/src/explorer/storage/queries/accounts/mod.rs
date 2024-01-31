use color_eyre::Result;
use sqlx::{query_as, Transaction};
use tracing::instrument;

use crate::explorer::storage::{models::accounts::Account, Storage};

impl Storage {
    #[instrument(skip(self))]
    pub async fn find_account_by_id(&self, id: i64) -> Result<Option<Account>> {
        Ok(query_as!(
            Account,
            r#"SELECT *
            FROM accounts
            WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    #[instrument(skip(self))]
    pub async fn find_account_by_fvm_address(
        &self,
        fvm_address: String,
    ) -> Result<Option<Account>> {
        Ok(query_as!(
            Account,
            r#"SELECT *
            FROM accounts
            WHERE fvm_address = ?"#,
            fvm_address
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    #[instrument(skip(self, db_tx))]
    pub async fn dbtx_find_account_by_fvm_address(
        &self,
        db_tx: &mut Transaction<'_, sqlx::Sqlite>,
        fvm_address: String,
    ) -> Result<Option<Account>> {
        Ok(query_as!(
            Account,
            r#"SELECT *
            FROM accounts
            WHERE fvm_address = ?"#,
            fvm_address
        )
        .fetch_optional(db_tx.as_mut())
        .await?)
    }

    #[instrument(skip(self))]
    pub async fn find_account_by_evm_address(
        &self,
        evm_address: String,
    ) -> Result<Option<Account>> {
        Ok(query_as!(
            Account,
            r#"SELECT *
            FROM accounts
            WHERE evm_address = ?"#,
            evm_address
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    #[instrument(skip(self, db_tx))]
    pub async fn dbtx_find_account_by_evm_address(
        &self,
        db_tx: &mut Transaction<'_, sqlx::Sqlite>,
        evm_address: String,
    ) -> Result<Option<Account>> {
        Ok(query_as!(
            Account,
            r#"SELECT *
            FROM accounts
            WHERE evm_address = ?"#,
            evm_address
        )
        .fetch_optional(db_tx.as_mut())
        .await?)
    }

    #[instrument(skip(self))]
    pub async fn find_account_by_svm_address(
        &self,
        svm_address: String,
    ) -> Result<Option<Account>> {
        Ok(query_as!(
            Account,
            r#"SELECT *
            FROM accounts
            WHERE svm_address = ?"#,
            svm_address
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    #[instrument(skip(self, db_tx))]
    pub async fn dbtx_find_account_by_svm_address(
        &self,
        db_tx: &mut Transaction<'_, sqlx::Sqlite>,
        svm_address: String,
    ) -> Result<Option<Account>> {
        Ok(query_as!(
            Account,
            r#"SELECT *
            FROM accounts
            WHERE svm_address = ?"#,
            svm_address
        )
        .fetch_optional(db_tx.as_mut())
        .await?)
    }

    #[instrument(skip(self, db_tx))]
    pub async fn insert_account(
        &self,
        db_tx: &mut Transaction<'_, sqlx::Sqlite>,
        account: &Account,
    ) -> Result<i64> {
        Ok(sqlx::query!(
            r#"INSERT INTO accounts (fvm_address, evm_address, svm_address, created_at_block_id, modified_at_block_id, executable_type, native_balance)
            VALUES (?,?,?,?,?,?,?)
            RETURNING id
            "#,
            account.fvm_address,
            account.evm_address,
            account.svm_address,
            account.created_at_block_id,
            account.modified_at_block_id,
            account.executable_type,
            account.native_balance,
        )
        .fetch_one(db_tx.as_mut())
        .await?.id)
    }

    #[instrument(skip(self, db_tx))]
    pub async fn update_account_balance(
        &self,
        db_tx: &mut Transaction<'_, sqlx::Sqlite>,
        account_id: i64,
        balance: String,
        block_id: i64,
    ) -> Result<()> {
        sqlx::query!(
            r#"UPDATE accounts
            SET native_balance = ?, modified_at_block_id = ?
            WHERE id = ?
            "#,
            balance,
            block_id,
            account_id,
        )
        .execute(db_tx.as_mut())
        .await?;

        Ok(())
    }

    #[instrument(skip(self, db_tx))]
    pub async fn update_account_executable_type(
        &self,
        db_tx: &mut Transaction<'_, sqlx::Sqlite>,
        account_id: i64,
        executable_type: Option<String>,
        block_id: i64,
    ) -> Result<()> {
        sqlx::query!(
            r#"UPDATE accounts
            SET executable_type = ?, modified_at_block_id = ?
            WHERE id = ?
            "#,
            executable_type,
            block_id,
            account_id,
        )
        .execute(db_tx.as_mut())
        .await?;

        Ok(())
    }
}
