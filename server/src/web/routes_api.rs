use super::Result;
use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct RpcInfo {
    pub id: Option<Value>,
    pub method: String,
}

pub fn routes() -> Router {
    Router::new()
        .route("/api/result", post(post_handler_test))
        .route("/api/res", get(get_handler_test))
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
