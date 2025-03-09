use crate::web::{Error, Result};
use lib_core::model::{user::UserForCreate, ModelManager};
use lib_core::service::{self, LoginPayload};
use lib_web::cookies::{remove_session_cookie, set_session_cookie};

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use tower_cookies::Cookies;
use tracing::debug;

pub fn routes(mm: ModelManager) -> Router {
    Router::new()
        .route("/api/login", post(login_handler))
        .route("/api/logout", post(logout_handler))
        .route("/api/register", post(register_handler))
        .route(
            "/api/validate_email/:web_token",
            post(validate_register_handler),
        )
        .with_state(mm)
}

// region:		=== Login ===

async fn login_handler(
    State(mm): State<ModelManager>,
    cookies: Cookies,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<Value>> {
    let session_auth = service::login(&mm, payload).await?;

    // Add token in cookies
    set_session_cookie(&cookies, session_auth.clone()).map_err(|_| Error::CannotSetCookie)?;

    debug!("{:<12} - Attemp successful", "LOGIN");

    let body = Json(json!({
        "result":{
            "Token_to_set_in_cookies": session_auth.token
        }
    }));

    Ok(body)
}

// endregion:	=== Login ===

// region:		=== Logout ===

#[derive(Debug, Deserialize)]
pub struct LogoutPayload {
    pub logout: bool,
}

pub async fn logout_handler(
    State(_mm): State<ModelManager>,
    cookies: Cookies,
    Json(payload): Json<LogoutPayload>,
) -> Result<Json<Value>> {
    debug!("{:<12} - api_logout_handler", "HANDLER");
    let should_logoff = payload.logout;

    if should_logoff {
        remove_session_cookie(&cookies).map_err(|_| Error::CannotRemoveCookie)?;
    }

    let body = Json(json!({
        "result": {
            "logged_out": should_logoff
        }
    }));

    Ok(body)
}

// endregion:	=== Logout ===

// region:		=== Register ===

pub async fn register_handler(
    State(mm): State<ModelManager>,
    Json(user_c): Json<UserForCreate>,
) -> Result<Json<Value>> {
    let web_token = service::register(&mm, user_c).await?;
    debug!("{:<12} - Email validation: {:#}", "REGISTER", web_token);

    let body = Json(json!({
        "result":{
            "Register success": true,
            "Next step":"Check your email"
        }
    }));

    Ok(body)
}

pub async fn validate_register_handler(
    State(mm): State<ModelManager>,
    cookies: Cookies,
    Path(web_token): Path<String>,
) -> Result<Json<Value>> {
    let session_auth = service::validate_email(&mm, &web_token).await?;

    // Add token in cookies
    set_session_cookie(&cookies, session_auth.clone()).map_err(|_| Error::CannotSetCookie)?;

    let body = Json(json!({
        "result":{
            "Email validated": true,
            "Message": "You are now logged in",
        }
    }));

    Ok(body)
}

// endregion:	=== Register ===
