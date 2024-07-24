mod config;
mod error;
mod web;

pub use self::error::{Error, Result};
use config::config;

use axum::Router;
use dotenv::dotenv;
use lib_core::model::{user::create_user_table, ModelManager};
use tracing::info;
use tracing_subscriber::EnvFilter;

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
    let (leptos_option, addr) = web::routes_leptos::get_leptos_config().await?;

    // Create MM (Database)
    let mm = ModelManager::new().await?;
    create_user_table(mm.clone()).await?;

    // region:        --- Axum router

    let routes_all = Router::new()
        .merge(web::routes_leptos::routes(leptos_option))
        .merge(web::routes_api::routes(mm.clone()));

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
