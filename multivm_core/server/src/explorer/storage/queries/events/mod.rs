use color_eyre::Result;
use sqlx::{query_as, Transaction};
use tracing::instrument;

use crate::explorer::storage::{models::events::Event, Storage};

impl Storage {
    #[instrument(skip(self))]
    pub async fn find_events_by_receipt_id(&self, receipt_id: i64) -> Result<Vec<Event>> {
        let mut events = query_as!(
            Event,
            r#"SELECT *
            FROM events
            WHERE receipt_id = ?"#,
            receipt_id
        )
        .fetch_all(&self.pool)
        .await?;

        events.sort_by(|a, b| a.index_in_receipt.cmp(&b.index_in_receipt));

        Ok(events)
    }

    #[instrument(skip(self))]
    pub async fn find_event_by_id(&self, id: i64) -> Result<Option<Event>> {
        Ok(query_as!(
            Event,
            r#"SELECT *
            FROM events
            WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    #[instrument(skip(self, db_tx))]
    pub async fn insert_event(
        &self,
        db_tx: &mut Transaction<'_, sqlx::Sqlite>,
        event: &Event,
    ) -> Result<i64> {
        Ok(sqlx::query!(
            r#"INSERT INTO events (receipt_id, message)
            VALUES (?, ?)
            RETURNING id
            "#,
            event.receipt_id,
            event.message,
        )
        .fetch_one(db_tx.as_mut())
        .await?
        .id)
    }
}
