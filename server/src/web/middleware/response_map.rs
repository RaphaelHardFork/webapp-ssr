use std::sync::Arc;

use axum::{
    response::{IntoResponse, Response},
    Json,
};
use leptos::ServerFnError;
use serde_json::{json, to_value};
use tracing::{debug, field::debug};

use crate::web;

pub async fn response_map_mw(res: Response) -> Response {
    debug!("{:<12} - response_map", "MIDDLEWARE");
    debug!("{:<12} - {:?}", "MW_RES", res.body());

    // get eventual error
    let server_fn_error = res.extensions().len();
    debug!("{:<12} - {:?}", "MW_RES", server_fn_error);
    let web_error = res.extensions().get::<Arc<web::Error>>().map(Arc::as_ref);
    let client_status_error = web_error.map(|we| we.client_status_and_error());

    // if client error, build new response
    let error_response = client_status_error
        .as_ref()
        .map(|(status_code, client_error)| {
            let client_error = to_value(client_error).ok();
            let message = client_error.as_ref().and_then(|v| v.get("message"));
            let detail = client_error.as_ref().and_then(|v| v.get("detail"));

            let client_error_body = json!({
              "error":{
                "message":message,
                "detail":detail
              }
            });

            debug!("{:<12} \n{}", "CLIENT ERROR BODY", client_error_body);
            (status_code.to_owned(), Json(client_error)).into_response()
        });

    // log request

    error_response.unwrap_or(res)
}
