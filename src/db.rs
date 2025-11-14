use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

use crate::error::AppError;

pub type DbPool = SqlitePool;

pub async fn init_pool(database_url: &str) -> Result<DbPool, AppError> {
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    Ok(pool)
}
