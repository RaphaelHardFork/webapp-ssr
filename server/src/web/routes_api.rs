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
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{debug, info};

#[derive(Debug, Deserialize)]
pub struct RpcInfo {
    pub id: Option<Value>,
    pub method: String,
}

pub fn routes(mm: ModelManager) -> Router {
    Router::new()
        .route("/api/result", post(post_handler_test))
        .route("/api/res", get(get_handler_test))
        .route("/api/user", post(create_user_handler))
        .route("/api/users", get(get_users_handler))
        .with_state(mm)
}

async fn post_handler_test(Json(rpc_info): Json<RpcInfo>) -> Result<Json<Value>> {
    info!("{:<12} - POST handler test", "HANDLER");
    let res = Json(json!({
        "id": rpc_info.id,
        "result": rpc_info.method
    }));

    Ok(res)
}

async fn get_handler_test() -> Result<Json<Value>> {
    info!("{:<12} - GET handler test", "HANDLER");
    let res = Json(json!({
        "id": 200,
        "result": "Test"
    }));

    Ok(res)
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
    debug!("{:<12} - create user", "API POST");
    match create_user(mm, &user.email, &user.pwd).await? {
        Some(id) => Ok(Json(json!({"result":{"success":true,"id":id}}))),
        None => Ok(Json(json!({"result":{"success":false}}))),
    }
}
