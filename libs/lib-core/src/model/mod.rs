mod base;
mod error;

pub mod session;
pub mod user;

pub use self::error::{Error, Result};

use crate::database::{new_db_pool, remove_db_file, Db};
use session::create_session_table;
use user::{create_user_table, UserBmc, UserForCreate};

#[derive(Debug, Clone)]
pub struct ModelManager {
    db: Db,
}

impl ModelManager {
    // constructor
    pub async fn new() -> Result<Self> {
        // FIXME: Only on dev mod => recreate DB in dev mod
        remove_db_file()?;

        let db = new_db_pool().await?;
        let mm = ModelManager { db };

        // Initial migrations (TODO: if not exist in PROD mode + sys_user:id:999)
        if !cfg!(test) {
            create_user_table(&mm).await?;
            let _ = UserBmc::create(
                &mm,
                UserForCreate {
                    username: "demo1".to_string(),
                    pwd_clear: "welcome".to_string(),
                    email: "demo@dev.com".to_string(),
                },
            )
            .await?;
            create_session_table(&mm).await?;
        }

        Ok(mm)
    }

    // properties accessor
    pub fn db(&self) -> &Db {
        &self.db
    }
}
