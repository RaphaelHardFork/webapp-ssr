use super::Result;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use lib_core::model::{
    user::{create_user, list_users, UserForCreate},
    ModelManager,
};

use serde_json::{json, Value};
use tracing::debug;

pub fn routes(mm: ModelManager) -> Router {
    Router::new()
        .route("/res/users", get(get_users_handler))
        .route("/res/user", post(create_user_handler))
        .with_state(mm)
}

async fn get_users_handler(State(mm): State<ModelManager>) -> Result<Json<Value>> {
    debug!("{:<12} - users", "API GET");
    let users = list_users(mm).await?;

    let body = Json(json!({
        "result":users
    }));

    Ok(body)
}

async fn create_user_handler(
    State(mm): State<ModelManager>,
    Json(user): Json<UserForCreate>,
) -> Result<Json<Value>> {
    debug!("{:<12} - user", "API POST");

    let id = create_user(mm, &user.email, &user.pwd).await?;

    let body = Json(json!({
        "result":id
    }));

    Ok(body)
}
