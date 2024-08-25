use lib_auth::token::session::generate_session_token;
use lib_utils::time::{format_time, now_utc};
use modql::field::{Field, Fields, HasFields};
use sea_query::{ColumnDef, Expr, ForeignKey, Iden, Query, SqliteQueryBuilder, Table};
use sea_query_binder::SqlxBinder;
use sqlx::prelude::FromRow;
use time::{Duration, OffsetDateTime};
use tracing::debug;
use uuid::Uuid;

use crate::model::{
    base::{
        utils::{add_timestamps_for_create, add_timestamps_for_update},
        TimestampIden,
    },
    user::UserIden,
};

use super::{base::DbBmc, Error, ModelManager, Result};

#[derive(FromRow, Fields)]
pub struct Session {
    pub id: i64,
    pub user_id: i64,
    pub token: String,
    pub session_type: bool, // Type session (7j) or privileged (10min) SQLite type/enum
    pub expiration: OffsetDateTime,
    // -------------- suggestion

    // ip_address: Option<String>, // check format
    // user_agent: Option<String>, // useful?
}

impl Session {
    pub fn session_type(&self) -> SessionType {
        SessionType::from(self.session_type)
    }
}

// region:		=== Const ===

const SESSION_DURATION_SEC: i64 = 604800; // 7 days
const SESSION_PRIVILEGED_DURATION_SEC: i64 = 600; // 10 min

pub enum SessionType {
    Session,
    Privileged,
}

impl SessionType {
    pub fn from(session_type: bool) -> Self {
        match session_type {
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
    session_type: bool,
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
    Table,
    Id,
    UserId,
    SessionType,
    Expiration,
    Token,
}

pub async fn create_session_table(mm: &ModelManager) -> Result<()> {
    // Build query
    let mut query = Table::create();
    query
        .table(SessionIden::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(SessionIden::Id)
                .big_integer()
                .not_null()
                .primary_key()
                .auto_increment(),
        )
        .col(ColumnDef::new(SessionIden::UserId).big_integer().not_null())
        .col(ColumnDef::new(SessionIden::Token).text().not_null())
        .col(
            ColumnDef::new(SessionIden::SessionType)
                .boolean()
                .not_null(),
        )
        .col(
            ColumnDef::new(SessionIden::Expiration)
                .text()
                .not_null()
                .default(Expr::current_timestamp()),
        )
        // timestamps
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
        )
        .foreign_key(
            ForeignKey::create()
                .from(SessionIden::Table, SessionIden::UserId)
                .to(UserIden::Table, UserIden::Id),
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

        // Generate expiration
        let now = now_utc();
        let expiration = match session_type {
            SessionType::Session => now + Duration::seconds(SESSION_DURATION_SEC),
            SessionType::Privileged => now + Duration::seconds(SESSION_PRIVILEGED_DURATION_SEC),
        };

        // Generate session token
        let token = generate_session_token(token_salt, session_type.is_privileged())?;

        // Extract and prepare fields
        let session_fi = SessionForInsert {
            user_id,
            session_type: session_type.is_privileged(),
            expiration: format_time(expiration),
            token,
        };

        let mut fields = session_fi.not_none_fields();
        add_timestamps_for_create(&mut fields, user_id);
        let (columns, sea_values) = fields.for_sea_insert();

        // Build query
        let mut query = Query::insert();
        query
            .into_table(SessionIden::Table)
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

    pub async fn get(mm: &ModelManager, token: &str) -> Result<Option<Session>> {
        // Build query
        let mut query = Query::select();
        query
            .from(SessionIden::Table)
            .columns(Session::field_column_refs())
            .and_where(Expr::col(SessionIden::Token).eq(token));
        let (sql, _) = query.build(SqliteQueryBuilder);

        // Execute query
        let session = sqlx::query_as::<_, Session>(&sql)
            .bind(token)
            .fetch_optional(mm.db())
            .await?;

        Ok(session)
    }

    pub async fn list(mm: &ModelManager, user_id: i64) -> Result<Vec<Session>> {
        let mut query = Query::select();
        query
            .from(SessionIden::Table)
            .columns(Session::field_column_refs());
        let (sql, _) = query.build(SqliteQueryBuilder);

        let sessions = sqlx::query_as::<_, Session>(&sql)
            .fetch_all(mm.db())
            .await?;

        Ok(sessions)
    }

    pub async fn extend_expiration(mm: &ModelManager, token: &str) -> Result<OffsetDateTime> {
        let session = Self::get(&mm, token).await?.ok_or(Error::NoAuthToken)?;
        // Update expiration
        let now = now_utc();
        let expiration = match SessionType::from(session.session_type) {
            SessionType::Session => now + Duration::seconds(SESSION_DURATION_SEC),
            SessionType::Privileged => now + Duration::seconds(SESSION_PRIVILEGED_DURATION_SEC),
        };

        let mut fields = Fields::new(vec![Field::new(
            SessionIden::Expiration,
            format_time(expiration).into(),
        )]);
        add_timestamps_for_update(&mut fields, session.user_id);
        let values = fields.for_sea_update();

        let mut query = Query::update();
        query
            .table(SessionIden::Table)
            .values(values)
            .and_where(Expr::col(SessionIden::Token).eq(token));
        let (sql, values) = query.build_sqlx(SqliteQueryBuilder);

        let _count = sqlx::query_with(&sql, values).execute(mm.db()).await?;

        Ok(expiration)
    }

    pub async fn delete(mm: &ModelManager, token: String) -> Result<()> {
        let mut query = Query::delete();
        query
            .from_table(SessionIden::Table)
            .and_where(Expr::col(SessionIden::Token).eq(token));
        let (sql, _) = query.build_sqlx(SqliteQueryBuilder);

        let _count = sqlx::query(&sql).execute(mm.db()).await?;

        Ok(())
    }
}

// D revok all

// endregion:	=== CRUD ===

// region:    --- Tests

#[cfg(test)]
mod tests {
    type Error = Box<dyn std::error::Error>;
    type Result<T> = core::result::Result<T, Error>; // For tests.

    use lib_utils::time::{parse_utc, Rfc3339};
    use sqlx::Row;

    use crate::model::user::{create_user_table, UserBmc, UserForAuth, UserForCreate};

    use super::*;

    #[tokio::test]
    async fn test_table() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;

        // -- Exec
        create_user_table(&mm).await?;
        create_session_table(&mm).await?;

        let rows = sqlx::query("PRAGMA table_info(session_iden)")
            .fetch_all(mm.db())
            .await?;

        // Display
        println!("Table info:");
        println!(
            "{:<10} {:<12} {:<10} {:<8} {}",
            "NAME", "TYPE", "NOT NULL", "PRIM_KEY", "DEFAULT_VALUE"
        );
        for row in &rows {
            let name: String = row.try_get("name")?;
            let col_type: String = row.try_get("type")?;
            let not_null: bool = row.try_get("notnull")?;
            let pk: bool = row.try_get("pk")?;
            let default: String = row.try_get("dflt_value")?;
            println!(
                "{:<10} {:<12} {:<10} {:<8} {}",
                name, col_type, not_null, pk, default
            );
        }
        assert_eq!(rows.len(), 5 + 4);

        let rows = sqlx::query("PRAGMA foreign_key_list(session_iden)")
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
        let user_id = UserBmc::create(
            &mm,
            UserForCreate {
                username: "Franck".to_string(),
                email: "franck@doma.in".to_string(),
                pwd: "welcome".to_string(),
            },
        )
        .await?;
        let user_auth: UserForAuth = UserBmc::get(&mm, user_id).await?;
        let token_salt = Uuid::parse_str(&user_auth.token_salt)?;

        // -- Exec
        let now = now_utc();
        let session_token = SessionBmc::create(
            &mm,
            token_salt,
            SessionForCreate {
                user_id,
                session_type: SessionType::Session,
            },
        )
        .await?;

        // -- Check
        if let Some(session) = SessionBmc::get(&mm, &session_token).await? {
            assert!(!session.session_type, "not privileged");
            assert_eq!(session_token, session.token);
            let estimated_exp =
                now.replace_millisecond(0)? + Duration::seconds(SESSION_DURATION_SEC);

            assert_eq!(estimated_exp, session.expiration.replace_millisecond(0)?);
        } else {
            assert!(false, "No session created");
        }

        let now = now_utc();
        let privileged_session_token = SessionBmc::create(
            &mm,
            token_salt,
            SessionForCreate {
                user_id,
                session_type: SessionType::Privileged,
            },
        )
        .await?;

        if let Some(privileged_session) = SessionBmc::get(&mm, &privileged_session_token).await? {
            assert!(privileged_session.session_type, "privileged");
            assert_eq!(privileged_session_token, privileged_session.token);
            let estimated_exp =
                now.replace_millisecond(0)? + Duration::seconds(SESSION_PRIVILEGED_DURATION_SEC);
            assert_eq!(
                estimated_exp,
                privileged_session.expiration.replace_millisecond(0)?
            );
        } else {
            assert!(false, "No session created");
        }

        Ok(())
    }
}

// endregion: --- Tests
