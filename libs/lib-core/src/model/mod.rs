pub mod session;
pub mod store;
pub mod user;

mod base;
mod error;

use crate::database::{new_db_pool, remove_db_file, Db};

pub use self::error::{Error, Result};

use axum::extract::FromRef;
use leptos::LeptosOptions;
use session::create_session_table;
use user::{create_user_table, UserBmc, UserForCreate};

// region:		=== AppState ===

#[derive(FromRef, Debug, Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub mm: ModelManager,
}

impl AppState {
    pub async fn new(leptos_options: LeptosOptions) -> Result<Self> {
        let mm = ModelManager::new().await?;

        // Initial migrations
        create_user_table(&mm).await?;
        let user_id = UserBmc::create(
            &mm,
            UserForCreate {
                username: "demo1".to_string(),
                pwd: "welcome".to_string(),
                email: "demo@dev.com".to_string(),
            },
        )
        .await?;
        create_session_table(&mm).await?;

        Ok(Self { leptos_options, mm })
    }
}

// endregion:	=== AppState ===

// region:		=== ModelManager ===

#[derive(Debug, Clone)]
pub struct ModelManager {
    db: Db,
}

impl ModelManager {
    // constructor
    pub async fn new() -> Result<Self> {
        // recreate DB in dev mod
        remove_db_file()?;

        let db = new_db_pool().await?;

        Ok(ModelManager { db })
    }

    // properties accessor
    pub fn db(&self) -> &Db {
        &self.db
    }
}

// endregion:	=== ModelManager ===
