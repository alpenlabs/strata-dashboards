use sqlx::{Pool, Postgres};
use anyhow::Result;

pub type DbPool = Pool<Postgres>;

pub async fn init_db_pool(db_url: &str) -> Result<Pool<Postgres>> {
    let pool = Pool::<Postgres>::connect(db_url).await?;
    // The `?` operator will return early if connect() fails, producing Err(...)
    Ok(pool)
}