use super::Result;
use axum::{routing::post, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct RpcInfo {
    pub id: Option<Value>,
    pub method: String,
}

pub fn routes() -> Router {
    Router::new().route("/api/result", post(rpc_axum_handler))
}

async fn rpc_axum_handler(Json(rpc_info): Json<RpcInfo>) -> Result<Json<Value>> {
    info!("{:<12} - api_login_handler", "HANDLER");
    let res = Json(json!({
        "id": rpc_info.id,
        "result": rpc_info.method
    }));

    Ok(res)
}
