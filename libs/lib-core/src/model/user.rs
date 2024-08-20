use std::{thread, time::Duration};

use super::{ModelManager, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tracing::debug;

// region:        --- Types

#[derive(FromRow, Serialize, Debug)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub pwd: String,
}

#[derive(Deserialize)]
pub struct UserForCreate {
    pub email: String,
    pub pwd: String,
}

// endregion:     --- Types

pub async fn create_user_table(mm: ModelManager) -> Result<()> {
    let db = mm.db;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS user (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email varchar(128) NOT NULL UNIQUE,
    pwd varchar(256)
    )",
    )
    .execute(&db)
    .await?;

    debug!("{:<12} - User table initiated", "DATABASE");

    Ok(())
}

pub async fn create_user(mm: ModelManager, email: &str, pwd: &str) -> Result<i64> {
    // wait 3s to simulate an error
    thread::sleep(Duration::from_millis(3000));

    let db = mm.db;
    let res = sqlx::query("INSERT INTO user (email, pwd) VALUES (?1, ?2)")
        .bind(email)
        .bind(pwd)
        .execute(&db)
        .await?;

    Ok(res.last_insert_rowid())
}

pub async fn list_users(mm: ModelManager) -> Result<Vec<User>> {
    let db = mm.db;
    debug!("HELLO");
    let users = sqlx::query_as::<_, User>("SELECT id, email, pwd FROM user")
        .fetch_all(&db)
        .await?;

    Ok(users)
}
