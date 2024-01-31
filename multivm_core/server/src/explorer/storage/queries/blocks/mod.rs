use color_eyre::Result;
use sqlx::{query_as, Transaction};
use tracing::instrument;

use crate::explorer::storage::{models::blocks::Block, Storage};

impl Storage {
    #[instrument(skip(self))]
    pub async fn find_block_by_number(&self, number: i64) -> Result<Option<Block>> {
        Ok(query_as!(
            Block,
            r#"SELECT *
            FROM blocks
            WHERE number = ?"#,
            number
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    #[instrument(skip(self))]
    pub async fn find_block_by_hash(&self, hash: &str) -> Result<Option<Block>> {
        Ok(query_as!(
            Block,
            r#"SELECT *
            FROM blocks
            WHERE hash = ?"#,
            hash
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    #[instrument(skip(self))]
    pub async fn find_block_by_id(&self, id: i64) -> Result<Option<Block>> {
        Ok(query_as!(
            Block,
            r#"SELECT *
            FROM blocks
            WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    #[instrument(skip(self))]
    pub async fn find_latest_block(&self) -> Result<Option<Block>> {
        Ok(query_as!(
            Block,
            r#"SELECT *
            FROM blocks
            ORDER BY number DESC
            LIMIT 1"#
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    #[instrument(skip(self))]
    pub async fn latest_blocks(&self, limit: i64) -> Result<Vec<Block>> {
        Ok(query_as!(
            Block,
            r#"SELECT *
            FROM blocks
            ORDER BY number DESC
            LIMIT ?"#,
            limit
        )
        .fetch_all(&self.pool)
        .await?)
    }

    #[instrument(skip(self, db_tx))]
    pub async fn insert_block(
        &self,
        db_tx: &mut Transaction<'_, sqlx::Sqlite>,
        block: &Block,
    ) -> Result<i64> {
        Ok(sqlx::query!(
            r#"INSERT INTO blocks (number, hash, timestamp, txs_count)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
            block.number,
            block.hash,
            block.timestamp,
            block.txs_count
        )
        .fetch_one(db_tx.as_mut())
        .await?
        .id)
    }
}
