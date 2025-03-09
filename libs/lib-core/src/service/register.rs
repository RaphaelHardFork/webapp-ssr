use lib_auth::token::{generate_web_token, validate_web_token, Token};

use crate::model::session::{SessionBmc, SessionForAuth, SessionForCreate, SessionType};
use crate::model::user::{UserBmc, UserForLogin};
use crate::model::{user::UserForCreate, ModelManager};

use crate::service::Result;

/// Validate input & returns the web_token to validate email
pub async fn register(mm: &ModelManager, user_c: UserForCreate) -> Result<Token> {
    user_c.validate()?;

    let user_id = UserBmc::create(&mm, user_c).await?;
    let user: UserForLogin = UserBmc::get(&mm, user_id).await?;

    let web_token = generate_web_token(&user.email, user.token_salt()?)?;

    Ok(web_token)
}

/// Validate email with JWT and returns a session to prevent another login
pub async fn validate_email(mm: &ModelManager, web_token: &str) -> Result<SessionForAuth> {
    let token: Token = web_token.parse()?;

    // get user
    let user: UserForLogin = UserBmc::first_by_identifier(mm, &token.ident).await?;

    // Validate web token before validate email
    validate_web_token(&token, user.token_salt()?)?;
    UserBmc::validate_email(mm, &token.ident).await?;

    // create session
    let session_c = SessionForCreate {
        user_id: user.id,
        session_type: SessionType::Session,
    };
    let session_id = SessionBmc::create(mm, user.token_salt()?, session_c).await?;
    let session: SessionForAuth = SessionBmc::get(mm, session_id).await?;

    Ok(session)
}

// region:    --- Tests

#[cfg(test)]
mod tests {
    type Error = Box<dyn std::error::Error>;
    type Result<T> = core::result::Result<T, Error>; // For tests.

    use crate::model::{
        session::create_session_table,
        user::{create_user_table, User},
    };
    use crate::service::Error as ServiceError;

    use super::*;

    #[tokio::test]
    async fn test_register_ok() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;
        create_user_table(&mm).await?;
        create_session_table(&mm).await?;
        let user_c = UserForCreate {
            username: "username_01".to_string(),
            email: "email_01@test.com".to_string(),
            pwd_clear: "password_01".to_string(),
        };

        // -- Exec
        let web_token = register(&mm, user_c.clone()).await?;
        assert_eq!(web_token.ident, user_c.email);

        // -- Check
        let user: User = UserBmc::first_by_identifier(&mm, &user_c.username).await?;
        assert_eq!(user.email, user_c.email);
        assert_eq!(user.username, user_c.username);

        Ok(())
    }

    #[tokio::test]
    async fn test_register_err() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;
        create_user_table(&mm).await?;
        create_session_table(&mm).await?;
        let user_c = UserForCreate {
            username: "username_01".to_string(),
            email: "email_01@test.com".to_string(),
            pwd_clear: "password_01".to_string(),
        };
        UserBmc::create(&mm, user_c.clone()).await?;

        // -- Exec
        assert!(matches!(
            register(&mm, user_c.clone()).await,
            Err(ServiceError::Model(_))
        ));

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_email_ok() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;
        create_user_table(&mm).await?;
        create_session_table(&mm).await?;
        let user_c = UserForCreate {
            username: "username_01".to_string(),
            email: "email_01@test.com".to_string(),
            pwd_clear: "password_01".to_string(),
        };
        let web_token = register(&mm, user_c.clone()).await?;

        // -- Exec
        let session_auth = validate_email(&mm, &format!("{:#}", web_token)).await?;
        assert!(!session_auth.privileged);

        Ok(())
    }
}

// endregion: --- Tests
