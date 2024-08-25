use super::{
    base::{DbBmc, UuidStr},
    ModelManager, Result,
};
use crate::model::{
    base::{
        utils::{add_timestamps_for_create, add_timestamps_for_update},
        CommonIden, TimestampIden,
    },
    Error,
};

use lazy_regex::regex_is_match;
use lib_auth::pwd::{self, ContentToHash};
use lib_utils::time::now_utc;
use modql::field::{Field, Fields, HasFields};
use sea_query::{ColumnDef, Cond, Expr, Iden, LogicalChainOper, Query, SqliteQueryBuilder, Table};
use sea_query_binder::SqlxBinder;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, FromRow};
use std::{thread, time::Duration};
use tracing::debug;
use uuid::Uuid;

#[derive(FromRow, Serialize, Debug, Clone, Fields)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
}

// region:		=== User variants ===

#[derive(Deserialize, Clone)]
pub struct UserForCreate {
    pub username: String,
    pub email: String,
    pub pwd: String,
}

#[derive(Fields, FromRow, Clone)]
struct UserForInsert {
    username: String,
    email: String,
    pwd_salt: UuidStr,
    token_salt: UuidStr,
}

#[derive(Clone, FromRow, Debug, Fields)]
pub struct UserForLogin {
    pub id: i64,
    pub username: Option<String>,
    pub email: Option<String>,

    pub pwd: String, // encrypted => #_scheme_id_#...
    pub pwd_salt: UuidStr,
    pub token_salt: UuidStr,
}

#[derive(Clone, FromRow, Debug, Fields)]
pub struct UserForAuth {
    pub id: i64,
    pub username: Option<String>,
    pub email: Option<String>,

    pub token_salt: UuidStr,
}

// endregion:	=== User variants ===

// region:		=== Controllers ===

impl UserForCreate {
    pub fn validate(&self) -> Result<()> {
        match self {
            _ if self.username.is_empty() => Err(Error::EmptyField { field: "username" }),
            _ if self.email.is_empty() => Err(Error::EmptyField { field: "email" }),
            _ if self.pwd.is_empty() => Err(Error::EmptyField { field: "pwd" }),
            user_c => {
                if !regex_is_match!(r"^[\w\.-]+@[a-zA-Z\d\.-]+\.[a-zA-Z]{2,}$", &user_c.email) {
                    Err(Error::WrongEmailFormat)
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl UserForLogin {
    pub fn pwd_salt(&self) -> Result<Uuid> {
        Uuid::parse_str(&self.pwd_salt).map_err(|ex| Error::UuidParsingFail(ex.to_string()))
    }

    pub fn token_salt(&self) -> Result<Uuid> {
        Uuid::parse_str(&self.token_salt).map_err(|ex| Error::UuidParsingFail(ex.to_string()))
    }
}

impl UserForAuth {
    pub fn token_salt(&self) -> Result<Uuid> {
        Uuid::parse_str(&self.token_salt).map_err(|ex| Error::UuidParsingFail(ex.to_string()))
    }
}

// endregion:	=== Controllers ===

// region:		=== User DB Table ===

#[derive(Iden)]
pub enum UserIden {
    Table,
    Id,
    // info
    Username,
    Email,
    // auth
    Pwd,
    PwdSalt,
    TokenSalt,
}

pub async fn create_user_table(mm: &ModelManager) -> Result<()> {
    // Check if the table exist (only for dev information) => by checking for the root user

    // Build query
    let mut query = Table::create();
    query
        .table(UserIden::Table)
        .if_not_exists()
        // id
        .col(
            ColumnDef::new(UserIden::Id)
                .big_integer()
                .not_null()
                .primary_key()
                .auto_increment(), // first user can have id 1000 to force starting at 1000
        )
        // info
        .col(
            ColumnDef::new(UserIden::Username)
                .string_len(128)
                .not_null()
                .unique_key(),
        )
        .col(
            ColumnDef::new(UserIden::Email)
                .string_len(128)
                .not_null()
                .unique_key(),
        )
        // auth
        .col(ColumnDef::new(UserIden::Pwd).string_len(256))
        .col(ColumnDef::new(UserIden::PwdSalt).text())
        .col(ColumnDef::new(UserIden::TokenSalt).text())
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
        );
    let sqlx_query = query.build(SqliteQueryBuilder);

    // Execute query
    sqlx::query(&sqlx_query).execute(mm.db()).await?;

    debug!("{:<12} - User table initiated", "DATABASE");

    Ok(())
}

// endregion:	=== User DB Table ===

/// Marker trait
pub trait UserBy: HasFields + for<'r> FromRow<'r, SqliteRow> + Unpin + Send {}
impl UserBy for User {}
impl UserBy for UserForLogin {}
impl UserBy for UserForAuth {}

// region:		=== User CRUD ===

pub struct UserBmc;
impl DbBmc for UserBmc {
    const TABLE: &'static str = "user";
}

impl UserBmc {
    pub async fn create(mm: &ModelManager, user_c: UserForCreate) -> Result<i64> {
        user_c.validate()?;
        let UserForCreate {
            username,
            email,
            pwd,
        } = user_c;

        // Generate salts (as SQlite does not have UUID, cannot be generated by default)
        let pwd_salt = Uuid::new_v4().to_string();
        let token_salt = Uuid::new_v4().to_string();
        let user_fi = UserForInsert {
            username,
            email,
            pwd_salt,
            token_salt,
        };

        // Extract and prepare Fields
        let mut fields = user_fi.clone().not_none_fields();
        add_timestamps_for_create(&mut fields, 999); // 999 = system ID
        let (columns, sea_values) = fields.for_sea_insert();

        // Build query
        let mut query = Query::insert();
        query
            .into_table(UserIden::Table)
            .columns(columns)
            .values(sea_values)?
            .returning(Query::returning().columns([CommonIden::Id]));
        let (sql, values) = query.build_sqlx(SqliteQueryBuilder);

        // Execute query
        let (user_id,) = sqlx::query_as_with::<_, (i64,), _>(&sql, values)
            .fetch_one(mm.db())
            .await?;

        // Create pwd_salt and hash pwd
        Self::update_pwd(mm, user_id, &pwd).await?;

        Ok(user_id)
    }

    pub async fn get<U>(mm: &ModelManager, id: i64) -> Result<U>
    where
        U: UserBy,
    {
        // Build query
        let mut query = Query::select();
        query
            .from(UserIden::Table)
            .columns(U::field_column_refs())
            .and_where(Expr::col(UserIden::Id).eq(id));
        let (sql, _) = query.build(SqliteQueryBuilder);

        // Execute
        let user = sqlx::query_as::<_, U>(&sql)
            .bind(id)
            .fetch_optional(mm.db())
            .await?
            .ok_or(Error::EntityNotFound {
                entity: Self::TABLE,
                id,
            })?;

        Ok(user)
    }

    pub async fn first_by_username<U>(mm: &ModelManager, username: &str) -> Result<Option<U>>
    where
        U: UserBy,
    {
        // Build query
        let mut query = Query::select();
        query
            .from(UserIden::Table)
            .columns(U::field_column_refs())
            .and_where(Expr::col(UserIden::Username).eq(username));
        let (sql, _) = query.build(SqliteQueryBuilder);

        // Execute
        let user = sqlx::query_as::<_, U>(&sql)
            .bind(username)
            .fetch_optional(mm.db())
            .await?;

        Ok(user)
    }

    pub async fn first_by_identifier<U>(mm: &ModelManager, identifier: &str) -> Result<Option<U>>
    where
        U: UserBy,
    {
        // Build query
        let mut query = Query::select();
        query
            .from(UserIden::Table)
            .columns(U::field_column_refs())
            .cond_where(
                Cond::any()
                    .add(Expr::col(UserIden::Email).eq(identifier))
                    .add(Expr::col(UserIden::Username).eq(identifier)),
            );
        let (sql, _) = query.build(SqliteQueryBuilder);

        // Execute
        let user = sqlx::query_as::<_, U>(&sql)
            .bind(identifier)
            .bind(identifier)
            .fetch_optional(mm.db())
            .await?;

        Ok(user)
    }

    pub async fn list(mm: &ModelManager) -> Result<Vec<User>> {
        let mut query = Query::select();
        query
            .from(UserIden::Table)
            .columns(User::field_column_refs());
        let (sql, _) = query.build(SqliteQueryBuilder);

        let users = sqlx::query_as::<_, User>(&sql).fetch_all(mm.db()).await?;

        Ok(users)
    }

    pub async fn update_pwd(mm: &ModelManager, id: i64, pwd: &str) -> Result<()> {
        // Get pwd_salt
        let user_fl: UserForLogin = Self::get(mm, id).await?;
        let pwd_salt = Uuid::parse_str(&user_fl.pwd_salt)
            .map_err(|ex| Error::UuidParsingFail(ex.to_string()))?;

        // Create pwd_hash
        let pwd_hash = pwd::hash_pwd(ContentToHash {
            content: pwd.to_string(),
            salt: pwd_salt,
        })
        .await?;

        // Extract and prepare fields
        let mut fields = Fields::new(vec![Field::new(UserIden::Pwd, pwd_hash.into())]);

        add_timestamps_for_update(&mut fields, id);
        let fields = fields.for_sea_update();

        // Build query
        let mut query = Query::update();
        query
            .table(UserIden::Table)
            .values(fields)
            .and_where(Expr::col(CommonIden::Id).eq(id));
        let (sql, values) = query.build_sqlx(SqliteQueryBuilder);

        // Execute query
        let _count = sqlx::query_with(&sql, values).execute(mm.db()).await?;
        // check if new pwd salt is generated ?

        Ok(())
    }
}

// endregion:	=== User CRUD ===

// region:		=== Tests ===

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Error as ModelError;
    use core::result::Result as CoreResult;
    use sqlx::Row;

    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Result<T> = CoreResult<T, Error>; // For tests.

    pub trait UnwrapOrClean<T> {
        async fn unwrap_or_clean(self, mm: &ModelManager) -> Result<T>;
    }
    impl<T, E> UnwrapOrClean<T> for CoreResult<T, E>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        async fn unwrap_or_clean(self, mm: &ModelManager) -> Result<T> {
            match self {
                Ok(val) => Ok(val),
                Err(err) => {
                    drop_user_table(mm).await?;
                    Err(err.into())
                }
            }
        }
    }

    async fn drop_user_table(mm: &ModelManager) -> Result<()> {
        sqlx::query("DROP TABLE IF EXISTS user_iden")
            .execute(mm.db())
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_create_user_table_ok() -> Result<()> {
        // Setup & Fixtures
        let mm = ModelManager::new().await?;

        // Exec
        create_user_table(&mm).await?;
        let rows = sqlx::query("PRAGMA table_info(user_iden)")
            .fetch_all(mm.db())
            .await?;

        // Display
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

        // Check
        assert_eq!(rows.len(), 10);

        // Clean
        drop_user_table(&mm).await?;

        // Check Clean (for others tests)
        let rows = sqlx::query("PRAGMA table_info(user_iden)")
            .fetch_all(mm.db())
            .await?;
        assert_eq!(rows.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_create_user_ok() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;
        create_user_table(&mm).await?;

        let user_c_valid = UserForCreate {
            username: "test___fx_username".to_string(),
            email: "test___fx_email@doma.in".to_string(),
            pwd: "test___fx_password".to_string(),
        };

        // -- Exec
        let user_id = UserBmc::create(&mm, user_c_valid.clone()).await?;

        // Check
        let user: User = UserBmc::get(&mm, user_id).await?;
        let user_login: UserForLogin = UserBmc::get(&mm, user_id).await?;

        // -- Check
        assert_eq!(user.id, user_id);
        assert_eq!(user.username, user_c_valid.username);
        assert_eq!(user.email, user_c_valid.email);

        assert_eq!(user_login.pwd_salt.len(), 32 + 4);
        assert_eq!(user_login.token_salt.len(), 32 + 4);
        assert!(user_login.pwd.starts_with("#02#$argon2d$"));

        // Clean
        drop_user_table(&mm).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_create_user_err_controller() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;
        create_user_table(&mm).await?;

        let empty = "".to_string();
        let valid_username = "test___fx_username".to_string();
        let valid_email = "test___fx_email@doma.in".to_string();
        let valid_pwd = "test___fx_pwd".to_string();

        let uc_fail_username = UserForCreate {
            username: empty.clone(),
            email: valid_email.clone(),
            pwd: valid_pwd.clone(),
        };
        let uc_fail_email = UserForCreate {
            username: valid_username.clone(),
            email: empty.clone(),
            pwd: valid_pwd.clone(),
        };
        let uc_fail_pwd = UserForCreate {
            username: valid_username.clone(),
            email: valid_email.clone(),
            pwd: empty.clone(),
        };

        let uc_fail_regex_email = UserForCreate {
            username: valid_username.clone(),
            email: "test___faulty_email".to_string(),
            pwd: valid_pwd.clone(),
        };

        // Exec & Check
        assert!(matches!(
            UserBmc::create(&mm, uc_fail_username).await,
            Err(ModelError::EmptyField { field: "username" })
        ));
        assert!(matches!(
            UserBmc::create(&mm, uc_fail_email).await,
            Err(ModelError::EmptyField { field: "email" })
        ));
        assert!(matches!(
            UserBmc::create(&mm, uc_fail_pwd).await,
            Err(ModelError::EmptyField { field: "pwd" })
        ));

        assert!(matches!(
            UserBmc::create(&mm, uc_fail_regex_email).await,
            Err(ModelError::WrongEmailFormat)
        ));

        // Clean
        drop_user_table(&mm).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_read_user_ok() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;
        create_user_table(&mm).await?;
        let user_c_valid_01 = UserForCreate {
            username: "test___fx01_username".to_string(),
            email: "test___fx01_email@doma.in".to_string(),
            pwd: "test___fx01_password".to_string(),
        };
        let user_c_valid_02 = UserForCreate {
            username: "test___fx02_username".to_string(),
            email: "test___fx02_email@doma.in".to_string(),
            pwd: "test___fx02_password".to_string(),
        };
        let user_c_valid_03 = UserForCreate {
            username: "test___fx03_username".to_string(),
            email: "test___fx03_email@doma.in".to_string(),
            pwd: "test___fx03_password".to_string(),
        };

        // Exec & Check
        let user_id01 = UserBmc::create(&mm, user_c_valid_01.clone()).await?;
        let user_id02 = UserBmc::create(&mm, user_c_valid_02.clone()).await?;
        let user_id03 = UserBmc::create(&mm, user_c_valid_03.clone()).await?;

        // get by identifier
        let user_by_username: Option<User> =
            UserBmc::first_by_identifier(&mm, "test___fx01_username").await?;
        let user_by_email: Option<User> =
            UserBmc::first_by_identifier(&mm, "test___fx01_email@doma.in").await?;

        assert!(user_by_username.is_some());
        assert!(user_by_email.is_some());

        // lists
        let users = UserBmc::list(&mm).await?;
        assert_eq!(users.len(), 3);

        // Clean
        drop_user_table(&mm).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_manual_sql() -> Result<()> {
        // -- Setup & Fixtures
        // let mm = ModelManager::new().await?;
        // create_user_table(&mm).await?;

        // let rows = sqlx::query("PRAGMA table_info(user_iden)")
        //     .fetch_all(mm.db())
        //     .await?;

        // for row in rows {
        //     let name: String = row.try_get("name")?;
        //     let col_type: String = row.try_get("type")?;
        //     let not_null: i32 = row.try_get("notnull")?;
        //     println!("{:?} {:?} {:?}", name, col_type, not_null);
        // }

        // sqlx::query("INSERT INTO user_iden (username, email, cid, mid) VALUES (?1, ?2, ?3, ?4)")
        //     .bind("demo".to_string())
        //     .bind("mail@demo.io".to_string())
        //     .bind(1)
        //     .bind(2)
        //     .execute(mm.db())
        //     .await?;

        // let users = sqlx::query_as::<_, User>("SELECT * FROM user_iden")
        //     .fetch_all(mm.db())
        //     .await?;

        // println!("{:?}", users);

        Ok(())
    }
}

// endregion:	=== Tests ===
