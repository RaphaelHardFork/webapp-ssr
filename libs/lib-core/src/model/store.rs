use super::{Error, Result};
use crate::config;
use lib_utils::files::create_file;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use tracing::debug;

pub type Db = Pool<Sqlite>;

pub async fn new_db_pool() -> Result<Db> {
    let max_connections = if cfg!(test) { 1 } else { 5 };
    let db_path = &config().DB_URL;
    if create_file(db_path.as_ref())? {
        debug!("{:<12} - New file created: {:?}", "DATABASE", db_path);
    }
    SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect(&format!("sqlite://{}", db_path))
        .await
        .map_err(|ex| Error::FailToCreatePool(ex.to_string()))
}
