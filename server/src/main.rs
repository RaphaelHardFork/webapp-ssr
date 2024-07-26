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
use leptos::{provide_context, LeptosOptions};
use leptos_axum::handle_server_fns_with_context;
use lib_core::model::{app_state::AppState, user::create_user_table, ModelManager};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;
use web::middleware::stamp::req_stamp;

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

    // Create AppState
    let app_state = AppState::new(leptos_options).await?;

    // region:        --- Axum router

    let routes_all = Router::new()
        .merge(web::routes_leptos::routes(app_state.clone()))
        .merge(web::routes_api::routes(app_state.mm.clone()))
        .layer(middleware::map_request(req_stamp));

    // endregion:     --- Axum router

    // region:        --- Start server

    // Ok to `unwrap` errors here
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("{:<12} - {:?}\n", "LISTENING", listener.local_addr());
    axum::serve(listener, routes_all.into_make_service())
        .await
        .unwrap();

    // endregion:     --- Start server

    Ok(())
}
