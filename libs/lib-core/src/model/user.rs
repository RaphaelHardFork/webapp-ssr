use crate::model::base::utils::{add_timestamps_for_create, add_timestamps_for_update};
use crate::model::base::{CommonIden, DbBmc, TimestampIden, UuidStr};
use crate::model::ModelManager;
use crate::model::{Error, Result};

use lazy_regex::regex_is_match;
use lib_auth::pwd::{self, ContentToHash};
use modql::field::{Field, Fields, HasFields};
use sea_query::{ColumnDef, Cond, Expr, Iden, Query, SqliteQueryBuilder, Table, Value};
use sea_query_binder::SqlxBinder;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, FromRow};
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
    pub pwd_clear: String,
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
    pub username: String,
    pub email: String,
    pub valid_email: bool,

    pub pwd: String,
    pub pwd_salt: UuidStr,
    pub token_salt: UuidStr,
}

#[derive(Clone, FromRow, Debug, Fields)]
pub struct UserForAuth {
    pub id: i64,
    pub username: String,
    pub email: String,

    pub token_salt: UuidStr,
}

/// Marker trait
pub trait UserBy: HasFields + for<'r> FromRow<'r, SqliteRow> + Unpin + Send {}
impl UserBy for User {}
impl UserBy for UserForLogin {}
impl UserBy for UserForAuth {}

// endregion:	=== User variants ===

// region:		=== Controllers & cast ===

impl UserForCreate {
    pub fn validate(&self) -> Result<()> {
        match self {
            _ if self.username.is_empty() => Err(Error::EmptyField { field: "username" }),
            _ if self.email.is_empty() => Err(Error::EmptyField { field: "email" }),
            _ if self.pwd_clear.is_empty() => Err(Error::EmptyField { field: "pwd" }),
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
        Ok(Uuid::parse_str(&self.pwd_salt)?)
    }

    pub fn token_salt(&self) -> Result<Uuid> {
        Ok(Uuid::parse_str(&self.token_salt)?)
    }
}

impl UserForAuth {
    pub fn token_salt(&self) -> Result<Uuid> {
        Ok(Uuid::parse_str(&self.token_salt)?)
    }
}

// endregion:	=== Controllers & cast ===

// region:		=== User DB Table ===

#[derive(Iden)]
pub enum UserIden {
    Id,
    // info
    Username,
    Email,
    ValidEmail,
    // auth
    Pwd,
    PwdSalt,
    TokenSalt,
}

pub async fn create_user_table(mm: &ModelManager) -> Result<()> {
    // Build query
    let mut query = Table::create();
    query
        .table(UserBmc::table_ref())
        .if_not_exists()
        // --- Id
        .col(
            ColumnDef::new(UserIden::Id)
                .big_integer()
                .not_null()
                .primary_key()
                .auto_increment(),
        )
        // --- Informations
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
        .col(
            ColumnDef::new(UserIden::ValidEmail)
                .boolean()
                .default(false),
        )
        // --- Auth
        .col(ColumnDef::new(UserIden::Pwd).string_len(256))
        .col(ColumnDef::new(UserIden::PwdSalt).text())
        .col(ColumnDef::new(UserIden::TokenSalt).text())
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

    debug!("{:<12} - User table initiated", "DATABASE");

    Ok(())
}

// endregion:	=== User DB Table ===

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
            pwd_clear,
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
        add_timestamps_for_create(&mut fields, 999); // FIXME: ROOt_CTX
        let (columns, sea_values) = fields.for_sea_insert();

        // Build query
        let mut query = Query::insert();
        query
            .into_table(Self::table_ref())
            .columns(columns)
            .values(sea_values)?
            .returning(Query::returning().columns([CommonIden::Id]));
        let (sql, values) = query.build_sqlx(SqliteQueryBuilder);

        // Execute query
        let (user_id,) = sqlx::query_as_with::<_, (i64,), _>(&sql, values)
            .fetch_one(mm.db())
            .await?;

        // Create pwd_salt and hash pwd
        Self::update_pwd(mm, user_id, &pwd_clear).await?;

        Ok(user_id)
    }

    pub async fn get<U>(mm: &ModelManager, id: i64) -> Result<U>
    where
        U: UserBy,
    {
        // Build query
        let mut query = Query::select();
        query
            .from(Self::table_ref())
            .columns(U::field_column_refs())
            .and_where(Expr::col(UserIden::Id).eq(id));
        let (sql, _) = query.build(SqliteQueryBuilder);

        // Execute
        let user = sqlx::query_as::<_, U>(&sql)
            .bind(id)
            .fetch_optional(mm.db())
            .await?
            .ok_or(Error::EntityIdNotFound {
                entity: Self::TABLE,
                id,
            })?;

        Ok(user)
    }

    pub async fn first_by_identifier<U>(mm: &ModelManager, identifier: &str) -> Result<U>
    where
        U: UserBy,
    {
        // Build query
        let mut query = Query::select();
        query
            .from(Self::table_ref())
            .columns(U::field_column_refs())
            .cond_where(
                Cond::any()
                    .add(Expr::col(UserIden::Email).eq(identifier))
                    .add(Expr::col(UserIden::Username).eq(identifier)),
            );
        let (sql, values) = query.build_sqlx(SqliteQueryBuilder);

        // Execute
        let user = sqlx::query_as_with::<_, U, _>(&sql, values)
            .fetch_optional(mm.db())
            .await?
            .ok_or(Error::EntityIdenNotFound {
                entity: Self::TABLE,
                identifier: identifier.to_string(),
            })?;

        Ok(user)
    }

    pub async fn list(mm: &ModelManager) -> Result<Vec<User>> {
        let mut query = Query::select();
        query
            .from(Self::table_ref())
            .columns(User::field_column_refs());
        let (sql, _) = query.build(SqliteQueryBuilder);

        let users = sqlx::query_as::<_, User>(&sql).fetch_all(mm.db()).await?;

        Ok(users)
    }

    pub async fn update_pwd(mm: &ModelManager, id: i64, pwd_clear: &str) -> Result<()> {
        // Get pwd_salt
        let user_login: UserForLogin = Self::get(mm, id).await?;
        let pwd_salt = Uuid::parse_str(&user_login.pwd_salt)?;

        // Create pwd_hash
        let pwd_hash = pwd::hash_pwd(ContentToHash {
            content: pwd_clear.to_string(),
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
            .table(Self::table_ref())
            .values(fields)
            .and_where(Expr::col(CommonIden::Id).eq(id));
        let (sql, values) = query.build_sqlx(SqliteQueryBuilder);

        // Execute query
        let _count = sqlx::query_with(&sql, values).execute(mm.db()).await?;

        Ok(())
    }

    pub async fn validate_email(mm: &ModelManager, user_email: &str) -> Result<()> {
        let user: UserForLogin = Self::first_by_identifier(&mm, &user_email).await?;

        if user.valid_email {
            return Err(Error::EmailAlreadyValiadted);
        }

        let mut fields = Fields::new(vec![Field::new(
            UserIden::ValidEmail,
            Value::Bool(Some(true)).into(),
        )]);
        add_timestamps_for_update(&mut fields, user.id);
        let fields = fields.for_sea_update();

        let mut query = Query::update();
        query
            .table(Self::table_ref())
            .values(fields)
            .and_where(Expr::col(CommonIden::Id).eq(user.id));
        let (sql, values) = query.build_sqlx(SqliteQueryBuilder);

        let _count = sqlx::query_with(&sql, values).execute(mm.db()).await?;

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

    // /// helpers to drop the table case the test exit
    // pub trait UnwrapOrClean<T> {
    //     async fn unwrap_or_clean(self, mm: &ModelManager) -> Result<T>;
    // }
    // impl<T, E> UnwrapOrClean<T> for CoreResult<T, E>
    // where
    //     E: std::error::Error + Send + Sync + 'static,
    // {
    //     async fn unwrap_or_clean(self, mm: &ModelManager) -> Result<T> {
    //         match self {
    //             Ok(val) => Ok(val),
    //             Err(err) => {
    //                 drop_user_table(mm).await?;
    //                 Err(err.into())
    //             }
    //         }
    //     }
    // }

    async fn drop_user_table(mm: &ModelManager) -> Result<()> {
        sqlx::query("DROP TABLE IF EXISTS user")
            .execute(mm.db())
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_table() -> Result<()> {
        // Setup & Fixtures
        let mm = ModelManager::new().await?;

        // Exec
        create_user_table(&mm).await?;
        let rows = sqlx::query("PRAGMA table_info(user)")
            .fetch_all(mm.db())
            .await?;

        // Display
        println!("\nTable 'user' info:");
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

        // Check
        assert_eq!(rows.len(), 7 + 4);

        // Clean
        drop_user_table(&mm).await?;

        // Check Clean (for others tests)
        let rows = sqlx::query("PRAGMA table_info(user)")
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
            pwd_clear: "test___fx_password".to_string(),
        };

        // -- Exec
        let user_id = UserBmc::create(&mm, user_c_valid.clone()).await?;

        // Check User
        let user: User = UserBmc::get(&mm, user_id).await?;
        assert_eq!(user.id, user_id);
        assert_eq!(user.username, user_c_valid.username);
        assert_eq!(user.email, user_c_valid.email);

        // Check UserForLogin
        let user: UserForLogin = UserBmc::get(&mm, user_id).await?;
        assert_eq!(user.id, user_id);
        assert_eq!(user.username, user_c_valid.username);
        assert_eq!(user.email, user_c_valid.email);
        assert_eq!(user.valid_email, false);
        assert_eq!(user.pwd_salt.len(), 32 + 4);
        assert_eq!(user.token_salt.len(), 32 + 4);
        assert!(user.pwd.starts_with("#02#$argon2d$"));

        // Check UserForAuth
        let user: UserForAuth = UserBmc::get(&mm, user_id).await?;
        assert_eq!(user.id, user_id);
        assert_eq!(user.username, user_c_valid.username);
        assert_eq!(user.email, user_c_valid.email);
        assert_eq!(user.token_salt.len(), 32 + 4);

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
            pwd_clear: valid_pwd.clone(),
        };
        let uc_fail_email = UserForCreate {
            username: valid_username.clone(),
            email: empty.clone(),
            pwd_clear: valid_pwd.clone(),
        };
        let uc_fail_pwd = UserForCreate {
            username: valid_username.clone(),
            email: valid_email.clone(),
            pwd_clear: empty.clone(),
        };

        let uc_fail_regex_email = UserForCreate {
            username: valid_username.clone(),
            email: "test___faulty_email".to_string(),
            pwd_clear: valid_pwd.clone(),
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

        Ok(())
    }

    #[tokio::test]
    async fn test_read_user_ok() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;
        create_user_table(&mm).await?;
        let valid_users: Vec<UserForCreate> = ["01", "02", "03"]
            .into_iter()
            .map(|i| UserForCreate {
                username: format!("username_{}", i),
                email: format!("email_{}@test.com", i),
                pwd_clear: format!("password_{}", i),
            })
            .collect();

        // Exec & Check
        let user_id01 = UserBmc::create(&mm, valid_users[0].clone()).await?;
        let user_id02 = UserBmc::create(&mm, valid_users[1].clone()).await?;
        let user_id03 = UserBmc::create(&mm, valid_users[2].clone()).await?;

        // Get by identifier
        let user01_by_username: User = UserBmc::first_by_identifier(&mm, "username_01").await?;
        let user02_by_email: User = UserBmc::first_by_identifier(&mm, "email_02@test.com").await?;
        assert_eq!(user01_by_username.id, user_id01);
        assert_eq!(user02_by_email.id, user_id02);

        // lists
        let users = UserBmc::list(&mm).await?;
        assert_eq!(users.len(), 3);
        assert_eq!(users[2].id, user_id03);

        Ok(())
    }

    #[tokio::test]
    async fn test_update_pwd_ok() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;
        create_user_table(&mm).await?;
        let valid_user = UserForCreate {
            username: format!("username_{}", "01"),
            email: format!("email_{}@test.com", "01"),
            pwd_clear: format!("password_{}", "01"),
        };
        let user_id = UserBmc::create(&mm, valid_user.clone()).await?;
        let user: UserForLogin = UserBmc::get(&mm, user_id).await?;

        // -- Exec
        UserBmc::update_pwd(&mm, user_id, "new password").await?;
        let user_pwd_updated: UserForLogin = UserBmc::get(&mm, user_id).await?;

        // -- Check
        assert_ne!(user.pwd, user_pwd_updated.pwd);
        assert_eq!(user.pwd_salt, user_pwd_updated.pwd_salt);

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_email_ok() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;
        create_user_table(&mm).await?;
        let valid_user = UserForCreate {
            username: format!("username_{}", "01"),
            email: format!("email_{}@test.com", "01"),
            pwd_clear: format!("password_{}", "01"),
        };
        let user_id = UserBmc::create(&mm, valid_user.clone()).await?;
        let user: UserForLogin = UserBmc::get(&mm, user_id).await?;

        // -- Exec
        UserBmc::validate_email(&mm, &user.email).await?;
        let user_email_validated: UserForLogin = UserBmc::get(&mm, user_id).await?;

        // -- Check
        assert!(!user.valid_email);
        assert!(user_email_validated.valid_email);

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_email_err() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;
        create_user_table(&mm).await?;
        let valid_user = UserForCreate {
            username: format!("username_{}", "01"),
            email: format!("email_{}@test.com", "01"),
            pwd_clear: format!("password_{}", "01"),
        };
        let user_id = UserBmc::create(&mm, valid_user.clone()).await?;
        let user: UserForLogin = UserBmc::get(&mm, user_id).await?;

        // Unknown email iden
        assert!(matches!(
            UserBmc::validate_email(&mm, "unknown_email").await,
            Err(ModelError::EntityIdenNotFound {
                entity: "user",
                identifier: _
            })
        ));

        // Mail already validated
        UserBmc::validate_email(&mm, &user.email).await?;
        assert!(matches!(
            UserBmc::validate_email(&mm, &user.email).await,
            Err(ModelError::EmailAlreadyValiadted)
        ));

        Ok(())
    }
}

// endregion:	=== Tests ===
