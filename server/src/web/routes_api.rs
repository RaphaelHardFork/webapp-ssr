use super::{middleware::auth::CtxW, Error, Result};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use lib_core::model::{
    session::{SessionBmc, SessionForCreate, SessionType},
    user::{UserBmc, UserForCreate, UserForLogin},
    ModelManager,
};

use serde_json::{json, Value};
use tracing::debug;

pub fn routes(mm: ModelManager) -> Router {
    Router::new()
        .route("/users", get(get_users_handler))
        .route("/user", post(create_user_handler))
        .with_state(mm)
}

async fn get_users_handler(State(mm): State<ModelManager>, ctx: CtxW) -> Result<Json<Value>> {
    debug!("{:<12} - users", "API GET");
    println!("==> Context: {:?}", ctx.0);
    let users = UserBmc::list(&mm).await?;

    let body = Json(json!({
        "result":users
    }));

    Ok(body)
}

async fn create_user_handler(
    State(mm): State<ModelManager>,
    Json(user_c): Json<UserForCreate>,
) -> Result<Json<Value>> {
    debug!("{:<12} - user", "API POST");

    let id = UserBmc::create(&mm, user_c).await?;

    let body = Json(json!({
        "result":id
    }));

    Ok(body)
}
