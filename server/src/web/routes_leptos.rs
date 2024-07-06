use super::{Error, Result};
use app::App;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, Response, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::response::Response as AxumResponse;
use axum::Router;
use leptos::{get_configuration, view, LeptosOptions};
use leptos_axum::{generate_route_list, LeptosRoutes};
use std::net::SocketAddr;
use tower::ServiceExt;
use tower_http::services::ServeDir;

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
    let req = Request::builder().uri(uri.clone()).body(Body::empty())?;

    match ServeDir::new(root).oneshot(req).await {
        Ok(res) => Ok(res.map(Body::new)),
        Err(_) => Err(Error::ServeDir),
    }
}

pub fn routes(leptos_options: LeptosOptions) -> Router {
    // generate HTML routes
    let routes = generate_route_list(App);

    // build router
    Router::new()
        .leptos_routes(&leptos_options, routes, App)
        .fallback(file_and_error_handler)
        .with_state(leptos_options)
}

pub async fn get_leptos_config() -> Result<(LeptosOptions, SocketAddr)> {
    let config = get_configuration(None).await?;
    let leptos_options = config.leptos_options;
    let addr = leptos_options.site_addr;

    Ok((leptos_options, addr))
}
