use super::{Error, Result};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use lib_auth::pwd::{validate_pwd, ContentToHash, SchemeStatus};
use lib_core::model::{
    session::{Session, SessionBmc, SessionForAuth, SessionForCreate, SessionType},
    user::{UserBmc, UserForCreate, UserForLogin},
    ModelManager,
};
use lib_utils::time::format_time;
use lib_web::cookies::{remove_session_cookie, set_session_cookie};
use serde::Deserialize;
use serde_json::{json, Value};
use tower_cookies::Cookies;
use tracing::debug;

pub fn routes(mm: ModelManager) -> Router {
    Router::new()
        .route("/api/login", post(login_handler))
        .route("/api/logout", post(logout_handler))
        .with_state(mm)
}

// region:		=== Login ===

#[derive(Deserialize)]
pub struct LoginPayload {
    pub username: Option<String>,
    pub email: Option<String>,
    pub pwd: String,
}

impl LoginPayload {
    pub fn validate(&self) -> Result<String> {
        match (&self.username, &self.email) {
            (Some(username), _) if !username.is_empty() => Ok(username.to_owned()),
            (_, Some(email)) if !email.is_empty() => Ok(email.to_owned()),
            _ => Err(Error::EmptyLoginPayload),
        }
    }
}

async fn login_handler(
    State(mm): State<ModelManager>,
    cookies: Cookies,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<Value>> {
    // Validate payload
    let identifier = payload.validate()?;

    // Get user
    let user: UserForLogin = UserBmc::first_by_identifier(&mm, &identifier)
        .await?
        .ok_or(Error::UserNotFound { identifier })?;

    // Validate pwd
    let scheme_status = validate_pwd(
        ContentToHash {
            content: payload.pwd,
            salt: user.pwd_salt()?,
        },
        &user.pwd,
    )
    .await
    .map_err(|_| Error::WrongPwd)?;

    // update pwd scheme if needed
    if let SchemeStatus::Outdated = scheme_status {
        debug!("pwd encrypt scheme outdating, upgrading");
        UserBmc::update_pwd(&mm, user.id, &user.pwd).await?;
    }

    // TODO: should list concerned token_session and update it or delete and create new
    // Create session token
    let session_c = SessionForCreate {
        user_id: user.id,
        session_type: SessionType::Session,
    };
    let session_token = SessionBmc::create(&mm, user.token_salt()?, session_c).await?;
    let Session {
        id,
        user_id,
        token,
        session_type,
        expiration,
    } = SessionBmc::get(&mm, &session_token)
        .await?
        .ok_or(Error::SessionNotFound {
            token: session_token.clone(),
        })?;

    // Add token in cookies
    set_session_cookie(
        &cookies,
        SessionForAuth {
            expiration: format_time(expiration),
            token,
            session_type,
        },
    )
    .map_err(|_| Error::CannotSetCookie)?;

    debug!("{:<12} - Attemp successful", "LOGIN");

    let body = Json(json!({
        "result":{
            "Token_to_set_in_cookies": session_token
        }
    }));

    Ok(body)
}

// endregion:	=== Login ===

#[derive(Debug, Deserialize)]
pub struct LogoffPayload {
    pub logout: bool,
}

pub async fn logout_handler(
    State(mm): State<ModelManager>,
    cookies: Cookies,
    Json(payload): Json<LogoffPayload>,
) -> Result<Json<Value>> {
    debug!("{:<12} - api_logoff_handler", "HANDLER");
    let should_logoff = payload.logout;

    if should_logoff {
        remove_session_cookie(&cookies).map_err(|_| Error::CannotRemoveCookie)?;
    }

    let body = Json(json!({
        "result": {
            "logged_off": should_logoff
        }
    }));

    Ok(body)
}
