use color_eyre::{eyre::Context, Result};
use sqlx::Transaction;
use tokio::fs::try_exists;
use tracing::instrument;

use super::config::StorageConfig;

pub mod models;
pub mod queries;

#[derive(Clone)]
pub struct Storage {
    pool: sqlx::Pool<sqlx::Sqlite>,
}

impl Storage {
    #[instrument]
    pub async fn new(config: StorageConfig) -> Result<Self> {
        let db_path = match config.sqlite_db_path {
            Some(db_path) => {
                if !try_exists(&db_path)
                    .await
                    .context("can't check if db exists")?
                {
                    tokio::fs::write(&db_path, [])
                        .await
                        .context("can't create db")?;
                }
                db_path
            }
            None => String::from(":memory:"),
        };

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect(&format!("sqlite:{}", db_path))
            .await
            .context("can't connect to db")?;

        sqlx::migrate!()
            .run(&pool)
            .await
            .context("can't run migrations")?;

        Ok(Self { pool })
    }

    pub async fn begin_transaction(&self) -> Result<Transaction<'_, sqlx::Sqlite>> {
        Ok(self.pool.begin().await?)
    }
}
