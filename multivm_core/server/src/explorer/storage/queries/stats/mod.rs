use color_eyre::Result;
use sqlx::{query_as, Transaction};
use tracing::instrument;

use crate::explorer::storage::{models::stats::Stats, Storage};

impl Storage {
    #[instrument(skip(self))]
    pub async fn find_latest_stats(&self) -> Result<Option<Stats>> {
        Ok(query_as!(
            Stats,
            r#"SELECT *
            FROM stats
            ORDER BY timestamp DESC
            LIMIT 1"#
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    #[instrument(skip(self, db_tx))]
    pub async fn insert_stats(
        &self,
        db_tx: &mut Transaction<'_, sqlx::Sqlite>,
        stats: &Stats,
    ) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO stats (timestamp, block_id, total_txs, total_accounts, total_contracts)
            VALUES (?, ?, ?, ?, ?)"#,
            stats.timestamp,
            stats.block_id,
            stats.total_txs,
            stats.total_accounts,
            stats.total_contracts,
        )
        .execute(db_tx.as_mut())
        .await?;

        Ok(())
    }
}
