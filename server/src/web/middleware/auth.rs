//! These middlewares ensure the Request contains the
//! appropriate session_token, else it returns an error.

use crate::web::{Error, Result};
use axum::{
    async_trait,
    body::Body,
    extract::{FromRequestParts, State},
    http::{request::Parts, Request},
    middleware::Next,
    response::Response,
};
use lib_auth::token::session::{validate_session_token, SessionToken};
use lib_core::{
    ctx::Ctx,
    model::{
        session::{SessionBmc, SessionForAuth, SessionType},
        user::{User, UserBmc, UserForAuth},
        ModelManager,
    },
};
use lib_utils::time::format_time;
use lib_web::cookies::{set_session_cookie, SESSION_TOKEN};
use serde::Serialize;
use tower_cookies::{Cookie, Cookies};

// region:		=== Context require (verify) ===

pub async fn ctx_require(ctx: Result<CtxW>, req: Request<Body>, next: Next) -> Result<Response> {
    // perform the resolution and
    ctx?;

    Ok(next.run(req).await)
}

// endregion:	=== Context require (verify) ===

// region:		=== Context resolver (insertion/creation) ===

/// Create the auth context for the request
pub async fn ctx_resolver(
    mm: State<ModelManager>,
    cookies: Cookies,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response> {
    let ctx_ext_result = _resolve_ctx(mm, &cookies).await;

    // Reset cookie if resolving context fail (execpt if there is not token)
    if ctx_ext_result.is_err() && !matches!(ctx_ext_result, Err(CtxExtError::TokenNotInCookie)) {
        cookies.remove(Cookie::from(SESSION_TOKEN))
    }

    // Add context in the request
    req.extensions_mut().insert(ctx_ext_result);

    Ok(next.run(req).await)
}

async fn _resolve_ctx(mm: State<ModelManager>, cookies: &Cookies) -> CtxExtResult {
    // get session_token from Cookies
    let session_token_str = cookies
        .get(SESSION_TOKEN)
        .map(|cookie| cookie.value().to_string())
        .ok_or(CtxExtError::TokenNotInCookie)?;

    // get user from session_token
    // => set user_id in session_token would allow to retrieve easely the user)
    // => Or a join between these two table can be made, but should be in a "use_case" or
    // "repository" folder, not in entities.
    let session = SessionBmc::first_by_token(&mm, &session_token_str)
        .await
        .map_err(|ex| CtxExtError::ModelAccessError(ex.to_string()))?;
    let user: UserForAuth = UserBmc::get(&mm, session.user_id)
        .await
        .map_err(|_| CtxExtError::UserNotFound)?;

    // Validate session_token
    let token_salt = user
        .token_salt()
        .map_err(|_| CtxExtError::UuidParsingFail)?;
    let session_token: SessionToken = session_token_str
        .parse()
        .map_err(|_| CtxExtError::TokenWrongFormat)?;
    validate_session_token(token_salt, session_token.clone())
        .map_err(|_| CtxExtError::FailValidate)?;

    // Extend token expiration
    let expiration = SessionBmc::extend_expiration(&mm, &session_token_str)
        .await
        .map_err(|_| CtxExtError::FailExtendExpiration)?;

    // update token in cookies
    let session_auth = SessionForAuth {
        token: session_token_str,
        privileged: session_token.privileged,
        expiration,
    };
    set_session_cookie(&cookies, session_auth).map_err(|_| CtxExtError::CannotSetInCookie)?;

    // Create context
    Ctx::new(user.id)
        .map(CtxW)
        .map_err(|ex| CtxExtError::CtxCreateFail(ex.to_string()))
}

// endregion:	=== Context resolver (creation) ===

// region:		=== Ctx Extractor ===

/// Context wrapper to implement extractor here
#[derive(Debug, Clone)]
pub struct CtxW(pub Ctx);

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for CtxW {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self> {
        parts
            .extensions
            .get::<CtxExtResult>()
            .ok_or(Error::CtxExt(CtxExtError::CtxNotInRequestExt))?
            .clone()
            .map_err(Error::CtxExt)
    }
}

// endregion:	=== Ctx Extractor ===

// region:		=== Ctx Error/Result ===

type CtxExtResult = core::result::Result<CtxW, CtxExtError>;

#[derive(Debug, Clone, Serialize)]
pub enum CtxExtError {
    TokenNotInCookie,
    ModelAccessError(String),
    SessionNotFound,
    UserNotFound,
    TokenWrongFormat,
    UuidParsingFail,
    FailValidate,
    FailExtendExpiration,
    CannotSetInCookie,
    CtxCreateFail(String),
    CtxNotInRequestExt,
}

// endregion:	=== Ctx Error/Result ===
