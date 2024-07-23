mod error;
pub mod store;
pub mod user;

use store::{new_db_pool, Db};

pub use self::error::{Error, Result};

#[derive(Clone)]
pub struct ModelManager {
    db: Db,
}

impl ModelManager {
    pub async fn new() -> Result<Self> {
        let db = new_db_pool().await?;

        Ok(ModelManager { db })
    }
}
