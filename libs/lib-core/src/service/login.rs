use crate::{
    model::{
        session::{SessionBmc, SessionForAuth, SessionForCreate, SessionType},
        user::{UserBmc, UserForLogin},
        ModelManager,
    },
    service::{Error, Result},
};
use lib_auth::pwd::{validate_pwd, ContentToHash, SchemeStatus};
use serde::Deserialize;
use tracing::debug;

#[derive(Deserialize, Clone)]
pub struct LoginPayload {
    pub username: Option<String>,
    pub email: Option<String>,
    pub pwd: String,
    pub privileged: Option<bool>,
}

impl LoginPayload {
    pub fn validate(&self) -> Result<String> {
        if self.pwd.is_empty() {
            return Err(Error::EmptyLoginPayload);
        }
        match (&self.username, &self.email) {
            (Some(username), _) if !username.is_empty() => Ok(username.to_owned()),
            (_, Some(email)) if !email.is_empty() => Ok(email.to_owned()),
            _ => Err(Error::EmptyLoginPayload),
        }
    }
}

/// Validate input & pwd and create a session
pub async fn login(mm: &ModelManager, login_payload: LoginPayload) -> Result<SessionForAuth> {
    let identifier = login_payload.validate()?;

    // Get user
    let user: UserForLogin = UserBmc::first_by_identifier(mm, &identifier).await?;

    // Validate pwd
    let scheme_status = validate_pwd(
        ContentToHash {
            content: login_payload.pwd,
            salt: user.pwd_salt()?,
        },
        &user.pwd,
    )
    .await?;

    // update pwd scheme if needed
    if let SchemeStatus::Outdated = scheme_status {
        debug!("pwd encrypt scheme outdating, upgrading");
        UserBmc::update_pwd(&mm, user.id, &user.pwd).await?;
    }

    // Create session
    let session_type = login_payload
        .privileged
        .map_or(SessionType::Session, |p| SessionType::from(p));
    let session_c = SessionForCreate {
        user_id: user.id,
        session_type,
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
        user::{create_user_table, UserForCreate},
    };
    use crate::service::Error as ServiceError;
    use lib_utils::time::{now_utc, parse_utc};
    use time::Duration;

    use super::*;

    #[tokio::test]
    async fn test_login_ok() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;
        create_user_table(&mm).await?;
        create_session_table(&mm).await?;
        let user_c = UserForCreate {
            username: "username_01".to_string(),
            email: "email_01@test.com".to_string(),
            pwd_clear: "password_01".to_string(),
        };
        let _ = UserBmc::create(&mm, user_c.clone()).await?;

        // Login with both
        let login_payload = LoginPayload {
            username: Some(user_c.username.clone()),
            email: Some(user_c.email.clone()),
            pwd: user_c.pwd_clear.clone(),
            privileged: None,
        };
        let expirate = now_utc() + Duration::days(7);
        let session_auth = login(&mm, login_payload.clone()).await?;
        assert_eq!(
            expirate.replace_second(0).unwrap().replace_millisecond(0),
            parse_utc(&session_auth.expiration)?
                .replace_second(0)
                .unwrap()
                .replace_millisecond(0)
        );
        assert!(!session_auth.privileged);

        // login with username
        let login_payload = LoginPayload {
            username: Some(user_c.username.clone()),
            email: None,
            pwd: user_c.pwd_clear.clone(),
            privileged: None,
        };
        let res = login(&mm, login_payload).await;
        assert!(res.is_ok());

        // login with email
        let login_payload = LoginPayload {
            username: None,
            email: Some(user_c.email),
            pwd: user_c.pwd_clear,
            privileged: None,
        };
        let res = login(&mm, login_payload).await;
        assert!(res.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_login_updating_pwd_ok() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;
        create_user_table(&mm).await?;
        create_session_table(&mm).await?;
        let user_c = UserForCreate {
            username: "username_01".to_string(),
            email: "email_01@test.com".to_string(),
            pwd_clear: "welcome".to_string(),
        };
        let user_id = UserBmc::create(&mm, user_c.clone()).await?;

        // Change manually pwd in DB with the previous scheme (precomputed with dev secret key)
        let pwd_salt = "550e8400-e29b-41d4-a716-446655440000";
        let scheme01_pwd_hash = "#01#6sNwjeQlw_EiT7BWZ54mPiKP-ShcJVGyXWGoMf2Fs-BJ0pd2ep9NAKgp2bVE-CRekR9TMsiFmJoXMvHwo6Mo5Q";

        let _ = sqlx::query("UPDATE user SET pwd_salt = ?1, pwd = ?2 WHERE id = ?3")
            .bind(pwd_salt)
            .bind(scheme01_pwd_hash)
            .bind(user_id)
            .execute(mm.db())
            .await?;
        let user: UserForLogin = UserBmc::get(&mm, user_id).await?;
        assert!(user.pwd.starts_with("#01#"));

        // Exec
        let login_payload = LoginPayload {
            username: Some(user_c.username),
            email: None,
            pwd: user_c.pwd_clear,
            privileged: None,
        };
        let _ = login(&mm, login_payload).await?;

        let user: UserForLogin = UserBmc::get(&mm, user_id).await?;
        assert!(user.pwd.starts_with("#02#"));

        Ok(())
    }

    #[tokio::test]
    async fn test_login_input_err() -> Result<()> {
        // -- Setup & Fixtures
        let mm = ModelManager::new().await?;
        create_user_table(&mm).await?;
        create_session_table(&mm).await?;
        let user_c = UserForCreate {
            username: "username_01".to_string(),
            email: "email_01@test.com".to_string(),
            pwd_clear: "password_01".to_string(),
        };
        let _ = UserBmc::create(&mm, user_c.clone()).await?;

        // Without identifier
        let login_payload = LoginPayload {
            username: None,
            email: None,
            pwd: user_c.pwd_clear.clone(),
            privileged: None,
        };
        assert!(matches!(
            login(&mm, login_payload).await,
            Err(ServiceError::EmptyLoginPayload)
        ));

        // Without identifier
        let login_payload = LoginPayload {
            username: Some(user_c.username.clone()),
            email: None,
            pwd: "".to_string(),
            privileged: None,
        };
        assert!(matches!(
            login(&mm, login_payload).await,
            Err(ServiceError::EmptyLoginPayload)
        ));

        // With inexistant username
        let login_payload = LoginPayload {
            username: Some("unknown user".to_string()),
            email: None,
            pwd: "password".to_string(),
            privileged: None,
        };
        assert!(matches!(
            login(&mm, login_payload).await,
            Err(ServiceError::Model(_))
        ));

        // With wrong password
        let login_payload = LoginPayload {
            username: Some(user_c.username),
            email: None,
            pwd: "wrong pwd".to_string(),
            privileged: None,
        };
        assert!(matches!(
            login(&mm, login_payload).await,
            Err(ServiceError::Pwd(_))
        ));

        Ok(())
    }
}

// endregion: --- Tests
