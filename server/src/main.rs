mod config;
mod error;
mod web;

pub use self::error::{Error, Result};

use axum::{
    body::Body,
    extract::{FromRef, Request, State},
    middleware,
    response::IntoResponse,
    Router,
};
use dotenv::dotenv;
use leptos::{leptos_config::Env, provide_context, LeptosOptions};
use leptos_axum::handle_server_fns_with_context;
use lib_core::model::{user::create_user_table, AppState, ModelManager};
use tower_cookies::CookieManagerLayer;
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;
use web::middleware::{
    auth::{ctx_require, ctx_resolver},
    response_map::response_map_mw,
    stamp::req_stamp,
};

#[tokio::main]
async fn main() -> Result<()> {
    // load .env (cargo leptos prevent to use .cargo/config.toml)
    dotenv().expect("Cannot load .env file");

    // create a global subscriber
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .without_time() // only on local deployments
        .with_target(false)
        .init();

    // get leptos config
    let (leptos_options, addr) = web::routes_leptos::get_leptos_config().await?;

    // Create AppState with dev/prod database
    let app_state = match leptos_options.env {
        Env::PROD => AppState::new(leptos_options).await?,
        Env::DEV => AppState::new(leptos_options).await?, // will change to a new DB
    };

    // region:        --- Axum router

    let auth_routes =
        web::routes_api::routes(app_state.mm.clone()).route_layer(middleware::from_fn(ctx_require));

    let routes_all = Router::new()
        .merge(web::routes_leptos::routes(app_state.clone()))
        // API side
        .nest("/auth", auth_routes)
        .merge(web::routes_login::routes(app_state.mm.clone()))
        // middleware and states
        .layer(middleware::from_fn_with_state(app_state.mm.clone(), ctx_resolver))
        .layer(middleware::map_response(response_map_mw))
        .layer(middleware::map_request(req_stamp))
        .layer(CookieManagerLayer::new())
        // rest of the router
        ;

    // endregion:     --- Axum router

    // region:        --- Start server

    // Ok to `unwrap` errors here
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!(
        "{:<12} - http://localhost:{}\n",
        "LISTENING ON",
        addr.port()
    );
    axum::serve(listener, routes_all.into_make_service())
        .await
        .unwrap();

    // endregion:     --- Start server

    Ok(())
}
