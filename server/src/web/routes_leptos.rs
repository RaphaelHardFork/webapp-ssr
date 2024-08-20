use crate::AppState;

use super::{Error, Result};
use app::App;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, Response, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::response::Response as AxumResponse;

use axum::routing::get;
use axum::{Json, Router};

use leptos::server_fn::middleware;
use leptos::{get_configuration, provide_context, view, LeptosOptions};
use leptos_axum::{generate_route_list, handle_server_fns_with_context, LeptosRoutes};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tower::ServiceExt;
use tower_http::services::ServeDir;
use tracing::debug;

// region:        --- Fallback

async fn file_and_error_handler(
    uri: Uri,
    State(options): State<LeptosOptions>,
    req: Request<Body>,
) -> AxumResponse {
    let root = options.site_root.clone();
    let res = get_static_file(uri.clone(), &root).await.unwrap();

    if res.status() == StatusCode::OK {
        res.into_response()
    } else {
        let handler =
            leptos_axum::render_app_to_stream(options.to_owned(), move || view! { <App/> });
        handler(req).await.into_response()
    }
}

async fn get_static_file(uri: Uri, root: &str) -> Result<Response<Body>> {
    let req = Request::builder()
        .uri(uri.clone())
        .body(Body::empty())
        .map_err(|e| Error::BuildAxumRequest(e.to_string()))?;

    match ServeDir::new(root).oneshot(req).await {
        Ok(res) => Ok(res.map(Body::new)),
        Err(_) => Err(Error::ServeDir),
    }
}

// endregion:     --- Fallback

// region:        --- Leptos handler

async fn server_fns_handler(
    State(app_state): State<AppState>,
    req: Request<Body>,
) -> impl IntoResponse {
    debug!("{:<12} - {} {}", "SERVER FN", req.method(), req.uri());

    handle_server_fns_with_context(
        move || {
            provide_context(app_state.clone());
        },
        req,
    )
    .await
}

pub async fn leptos_routes_handler(
    State(app_state): State<AppState>,
    req: Request<Body>,
) -> AxumResponse {
    debug!("{:<12} - {} {}", "BROWSER REQ", req.method(), req.uri());

    let handler = leptos_axum::render_app_to_stream_with_context(
        app_state.leptos_options.clone(),
        move || provide_context(app_state.clone()),
        move || view! { <App/> },
    );
    handler(req).await.into_response()
}

// endregion:     --- Leptos handler

pub fn routes(app_state: AppState) -> Router {
    // generate HTML routes
    let routes = generate_route_list(App);

    // build router
    Router::new()
        .route(
            "/api/*fn_name",
            get(server_fns_handler).post(server_fns_handler),
        )
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .fallback(file_and_error_handler)
        .with_state(app_state)
}

pub async fn get_leptos_config() -> Result<(LeptosOptions, SocketAddr)> {
    let config = get_configuration(None)
        .await
        .map_err(|e| Error::GetLeptosConfig(e.to_string()))?;
    let leptos_options = config.leptos_options;
    let addr = leptos_options.site_addr;

    Ok((leptos_options, addr))
}
