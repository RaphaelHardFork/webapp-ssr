use crate::model::base::utils::{add_timestamps_for_create, add_timestamps_for_update};
use crate::model::base::{DbBmc, TimestampIden};
use crate::model::user::{UserBmc, UserIden};
use crate::model::ModelManager;
use crate::model::{Error, Result};

use lib_auth::token::session::generate_session_token;
use lib_utils::time::{format_time, now_utc};
use modql::field::{Field, Fields, HasFields};
use sea_query::{ColumnDef, Expr, ForeignKey, Iden, Query, SqliteQueryBuilder, Table};
use sea_query_binder::SqlxBinder;
use sqlx::prelude::FromRow;
use time::{Duration, OffsetDateTime};
use tracing::debug;
use uuid::Uuid;

/// User sessions, token is stored into cookies.
/// IP addr and browser user agent could be implemented
/// as well to increase security but reduce privacy (or should be hashed).
///
/// Session type (privileged) is stored as bool as there is not type in SQlite
#[derive(FromRow, Fields)]
pub struct Session {
    pub id: i64,
    pub user_id: i64,
    pub token: String,
    pub privileged: bool,
    pub expiration: OffsetDateTime,
}

// region:		=== Const ===

const SESSION_DURATION_SEC: i64 = 604800; // 7 days
const SESSION_PRIVILEGED_DURATION_SEC: i64 = 600; // 10 min

pub enum SessionType {
    Session,
    Privileged,
}

impl Session {
    pub fn session_type(&self) -> SessionType {
        SessionType::from(self.privileged)
    }
}

impl SessionType {
    pub fn from(is_privileged: bool) -> Self {
        match is_privileged {
            false => Self::Session,
            true => Self::Privileged,
        }
    }

    pub fn is_privileged(&self) -> bool {
        match &self {
            Self::Session => false,
            Self::Privileged => true,
        }
    }
}

// endregion:	=== Const ===

// region:		=== Session variants ===

pub struct SessionForCreate {
    pub user_id: i64,
    pub session_type: SessionType,
}

#[derive(Fields, FromRow, Debug)]
pub struct SessionForInsert {
    user_id: i64,
    token: String,
    privileged: bool,
    expiration: String,
}

#[derive(Fields, FromRow, Debug)]
pub struct SessionForAuth {
    pub token: String,
    pub session_type: bool,
    pub expiration: String,
}

// endregion:	=== Session variants ===

// region:		=== Session DB table ===

#[derive(Iden)]
pub enum SessionIden {
    Id,
    UserId,
    Privileged,
    Expiration,
    Token,
}

pub async fn create_session_table(mm: &ModelManager) -> Result<()> {
    // Build query
    let mut query = Table::create();
    query
        .table(SessionBmc::table_ref())
        .if_not_exists()
        // --- Keys
        .col(
            ColumnDef::new(SessionIden::Id)
                .big_integer()
                .not_null()
                .primary_key()
                .auto_increment(),
        )
        .col(ColumnDef::new(SessionIden::UserId).big_integer().not_null())
        .foreign_key(
            ForeignKey::create()
                .from(SessionBmc::table_ref(), SessionIden::UserId)
                .to(UserBmc::table_ref(), UserIden::Id),
        )
        // --- Content
        .col(ColumnDef::new(SessionIden::Token).text().not_null())
        .col(ColumnDef::new(SessionIden::Privileged).boolean().not_null())
        .col(
            ColumnDef::new(SessionIden::Expiration)
                .text()
                .not_null()
                .default(Expr::current_timestamp()),
        )
        // --- Timestamps
        .col(ColumnDef::new(TimestampIden::CId).big_integer().not_null())
        .col(
            ColumnDef::new(TimestampIden::CTime)
                .text()
                .not_null()
                .default(Expr::current_timestamp()),
        )
        .col(ColumnDef::new(TimestampIden::MId).big_integer().not_null())
        .col(
            ColumnDef::new(TimestampIden::MTime)
                .text()
                .not_null()
                .default(Expr::current_timestamp()),
        );
    let sqlx_query = query.build(SqliteQueryBuilder);

    // Execute query
    sqlx::query(&sqlx_query).execute(mm.db()).await?;

    debug!("{:<12} - Session table initiated", "DATABASE");

    Ok(())
}

// endregion:	=== Session DB table ===

// region:		=== CRUD ===

pub struct SessionBmc;
impl DbBmc for SessionBmc {
    const TABLE: &'static str = "session";
}

impl SessionBmc {
    pub async fn create(
        mm: &ModelManager,
        token_salt: Uuid,
        session_c: SessionForCreate,
    ) -> Result<String> {
        let SessionForCreate {
            user_id,
            session_type,
        } = session_c;
        let privileged = session_type.is_privileged();

        // Generate expiration
        let now = now_utc();
        let expiration = match session_type {
            SessionType::Session => now + Duration::seconds(SESSION_DURATION_SEC),
            SessionType::Privileged => now + Duration::seconds(SESSION_PRIVILEGED_DURATION_SEC),
        };
        let expiration = format_time(expiration);

        // Generate session token
        let token = generate_session_token(token_salt, privileged)?;

        // Extract and prepare fields
        let session_fi = SessionForInsert {
            user_id,
            privileged,
            expiration,
            token,
        };

        let mut fields = session_fi.not_none_fields();
        add_timestamps_for_create(&mut fields, user_id);
        let (columns, sea_values) = fields.for_sea_insert();

        // Build query
        let mut query = Query::insert();
        query
            .into_table(Self::table_ref())
            .columns(columns)
            .values(sea_values)?
            .returning(Query::returning().columns([SessionIden::Token]));
        let (sql, values) = query.build_sqlx(SqliteQueryBuilder);

        // Execute query
        let (token,) = sqlx::query_as_with::<_, (String,), _>(&sql, values)
            .fetch_one(mm.db())
            .await?;

        Ok(token)
    }

    pub async fn get(mm: &ModelManager, token: &str) -> Result<Session> {
        // Build query
        let mut query = Query::select();
        query
            .from(Self::table_ref())
            .columns(Session::field_column_refs())
            .and_where(Expr::col(SessionIden::Token).eq(token));
        let (sql, _) = query.build(SqliteQueryBuilder);

        // Execute query
        let session = sqlx::query_as::<_, Session>(&sql)
            .bind(token)
            .fetch_optional(mm.db())
            .await?
            .ok_or(Error::EntityIdenNotFound {
                entity: Self::TABLE,
                identifier: token.to_string(),
            })?;

        Ok(session)
    }

    pub async fn list(mm: &ModelManager, user_id: i64) -> Result<Vec<Session>> {
        let mut query = Query::select();
        query
            .from(Self::table_ref())
            .columns(Session::field_column_refs())
            .and_where(Expr::col(SessionIden::UserId).eq(user_id));
        let (sql, values) = query.build_sqlx(SqliteQueryBuilder);

        let sessions = sqlx::query_as_with::<_, Session, _>(&sql, values)
            .fetch_all(mm.db())
            .await?;

        Ok(sessions)
    }

    /// Returns the expiration formatted RFC 3339
    pub async fn extend_expiration(mm: &ModelManager, token: &str) -> Result<String> {
        let session = Self::get(&mm, token).await?;

        // Update expiration
        let now = now_utc();
        let expiration = match SessionType::from(session.privileged) {
            SessionType::Session => now + Duration::seconds(SESSION_DURATION_SEC),
            SessionType::Privileged => now + Duration::seconds(SESSION_PRIVILEGED_DURATION_SEC),
        };
        let expiration = format_time(expiration);

        let mut fields = Fields::new(vec![Field::new(
            SessionIden::Expiration,
            expiration.clone().into(),
        )]);
        add_timestamps_for_update(&mut fields, session.user_id);
        let values = fields.for_sea_update();

        let mut query = Query::update();
        query
            .table(Self::table_ref())
            .values(values)
            .and_where(Expr::col(SessionIden::Token).eq(token));
        let (sql, values) = query.build_sqlx(SqliteQueryBuilder);

        let _count = sqlx::query_with(&sql, values).execute(mm.db()).await?;

        Ok(expiration)
    }

    pub async fn delete(mm: &ModelManager, token: String) -> Result<()> {
        let mut query = Query::delete();
        query
            .from_table(Self::table_ref())
            .and_where(Expr::col(SessionIden::Token).eq(token));
        let (sql, values) = query.build_sqlx(SqliteQueryBuilder);

        let _count = sqlx::query_with(&sql, values).execute(mm.db()).await?;

        Ok(())
    }
}

// TODO: revoke all

// endregion:	=== CRUD ===

// region:    --- Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::user::{create_user_table, UserBmc, UserForAuth, UserForCreate};
    use crate::model::Error as ModelError;
    use lib_utils::time::parse_utc;
    use sqlx::Row;

    type Error = Box<dyn std::error::Error>;
    type Result<T> = core::result::Result<T, Error>; // For tests.

    #[tokio::test]
    async fn test_table() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;

        // -- Exec
        create_user_table(&mm).await?;
        create_session_table(&mm).await?;

        let rows = sqlx::query("PRAGMA table_info(session)")
            .fetch_all(mm.db())
            .await?;

        // Display
        println!("\nTable 'session' info:");
        println!(
            "{:<12} {:<12} {:<10} {:<8} {}",
            "NAME", "TYPE", "NOT NULL", "PRIM_KEY", "DEFAULT_VALUE"
        );
        for row in &rows {
            let name: String = row.try_get("name")?;
            let col_type: String = row.try_get("type")?;
            let not_null: bool = row.try_get("notnull")?;
            let pk: bool = row.try_get("pk")?;
            let default: String = row.try_get("dflt_value")?;
            println!(
                "{:<12} {:<12} {:<10} {:<8} {}",
                name, col_type, not_null, pk, default
            );
        }
        assert_eq!(rows.len(), 5 + 4);

        let rows = sqlx::query("PRAGMA foreign_key_list(session)")
            .fetch_all(mm.db())
            .await?;

        println!("\nForeign Keys:");
        println!(
            "{:<3} ({:<4}) {:<8} {:<10} {:<8} - {:<8} | {:<8}",
            "ID", "SEQ", "FROM", "TO TABLE", "COLUMN", "ON_UPDATE", "ON_DELETE"
        );
        for row in &rows {
            let id: i32 = row.try_get("id")?;
            let seq: i32 = row.try_get("seq")?;
            let table_name: String = row.try_get("table")?;
            let from_column: String = row.try_get("from")?;
            let to_column: String = row.try_get("to")?;
            let on_update: String = row.try_get("on_update")?;
            let on_delete: String = row.try_get("on_delete")?;
            println!(
                "{:<3} ({:<4}) {:<8} {:<10} {:<8} - {:<8} | {:<8}",
                id, seq, from_column, table_name, to_column, on_update, on_delete
            );
        }
        assert_eq!(rows.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_create_session_ok() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;
        create_user_table(&mm).await?;
        create_session_table(&mm).await?;

        let valid_users: Vec<UserForCreate> = ["01", "02"]
            .into_iter()
            .map(|i| UserForCreate {
                username: format!("username_{}", i),
                email: format!("email_{}@test.com", i),
                pwd_clear: format!("password_{}", i),
            })
            .collect();

        let user01_id = UserBmc::create(&mm, valid_users[0].clone()).await?;
        let user02_id = UserBmc::create(&mm, valid_users[1].clone()).await?;

        let user01: UserForAuth = UserBmc::get(&mm, user01_id).await?;
        let user02: UserForAuth = UserBmc::get(&mm, user02_id).await?;

        // Normal session
        let now = now_utc();
        let session_token = SessionBmc::create(
            &mm,
            user01.token_salt()?,
            SessionForCreate {
                user_id: user01_id,
                session_type: SessionType::Session,
            },
        )
        .await?;

        // -- Check
        let session = SessionBmc::get(&mm, &session_token).await?;
        assert!(!session.privileged, "not privileged");
        assert_eq!(session_token, session.token);

        let estimated_exp = now.replace_millisecond(0)? + Duration::seconds(SESSION_DURATION_SEC);
        assert_eq!(estimated_exp, session.expiration.replace_millisecond(0)?);

        // Privileged session
        let now = now_utc();
        let privileged_session_token = SessionBmc::create(
            &mm,
            user01.token_salt()?,
            SessionForCreate {
                user_id: user01_id,
                session_type: SessionType::Privileged,
            },
        )
        .await?;

        let privileged_session = SessionBmc::get(&mm, &privileged_session_token).await?;
        assert!(privileged_session.privileged, "privileged");
        assert_eq!(privileged_session_token, privileged_session.token);

        let estimated_exp =
            now.replace_millisecond(0)? + Duration::seconds(SESSION_PRIVILEGED_DURATION_SEC);
        assert_eq!(
            estimated_exp,
            privileged_session.expiration.replace_millisecond(0)?
        );

        let _ = SessionBmc::create(
            &mm,
            user02.token_salt()?,
            SessionForCreate {
                user_id: user02_id,
                session_type: SessionType::Session,
            },
        )
        .await?;
        let sessions = SessionBmc::list(&mm, user01.id).await?;
        assert_eq!(sessions.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_extend_expiration_ok() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;
        create_user_table(&mm).await?;
        create_session_table(&mm).await?;

        let user_c = UserForCreate {
            username: format!("username_{}", "01"),
            email: format!("email_{}@test.com", "01"),
            pwd_clear: format!("password_{}", "01"),
        };
        let user_id = UserBmc::create(&mm, user_c.clone()).await?;
        let user: UserForAuth = UserBmc::get(&mm, user_id).await?;

        let session_token = SessionBmc::create(
            &mm,
            user.token_salt()?,
            SessionForCreate {
                user_id,
                session_type: SessionType::Session,
            },
        )
        .await?;
        let session = SessionBmc::get(&mm, &session_token).await?;

        // -- Exec
        let new_expiration = SessionBmc::extend_expiration(&mm, &session_token).await?;
        let new_expiration = parse_utc(&new_expiration)?;

        // -- Check
        assert_ne!(session.expiration, new_expiration);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_ok() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;
        create_user_table(&mm).await?;
        create_session_table(&mm).await?;

        let user_c = UserForCreate {
            username: format!("username_{}", "01"),
            email: format!("email_{}@test.com", "01"),
            pwd_clear: format!("password_{}", "01"),
        };
        let user_id = UserBmc::create(&mm, user_c.clone()).await?;
        let user: UserForAuth = UserBmc::get(&mm, user_id).await?;

        let session_token01 = SessionBmc::create(
            &mm,
            user.token_salt()?,
            SessionForCreate {
                user_id,
                session_type: SessionType::Session,
            },
        )
        .await?;
        let session_token02 = SessionBmc::create(
            &mm,
            user.token_salt()?,
            SessionForCreate {
                user_id,
                session_type: SessionType::Privileged,
            },
        )
        .await?;
        let _session01 = SessionBmc::get(&mm, &session_token01).await?;
        let session02 = SessionBmc::get(&mm, &session_token02).await?;

        // -- Exec
        SessionBmc::delete(&mm, session_token01.clone()).await?;

        // -- Check
        assert!(matches!(
            SessionBmc::get(&mm, &session_token01).await,
            Err(ModelError::EntityIdenNotFound {
                entity: "session",
                identifier: _
            })
        ));
        let session02_again = SessionBmc::get(&mm, &session_token02).await?;
        assert_eq!(session02.id, session02_again.id);

        Ok(())
    }
}

// endregion: --- Tests
