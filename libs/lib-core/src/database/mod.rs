mod error;

pub use error::{Error, Result};

use crate::config;
use lib_utils::files::{create_file, delete_file};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use tracing::debug;

// region:		=== DB connection ===

pub type Db = Pool<Sqlite>;

pub async fn new_db_pool() -> Result<Db> {
    let max_connections = if cfg!(test) { 1 } else { 5 };

    let db_url = if cfg!(test) {
        "sqlite::memory:".to_string()
    } else {
        let db_path = config().DB_URL.clone();
        if create_file(db_path.as_ref())? {
            debug!("{:<12} - New file created: {:?}", "DATABASE", db_path);
        }
        format!("sqlite://{}", db_path)
    };

    // Create DB pool
    SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect(&db_url)
        .await
        .map_err(|ex| Error::FailToCreatePool(ex.to_string()))
}

// endregion:	=== DB connection ===

pub fn remove_db_file() -> Result<()> {
    debug!("{:<12} - Remove old DB: {:?}", "DATABASE", config().DB_URL);
    delete_file(config().DB_URL.as_ref())?;
    Ok(())
}
