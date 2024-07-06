mod web;

use axum::Router;
use tracing::info;
use tracing_subscriber::EnvFilter;
use web::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // create a global subscriber
    tracing_subscriber::fmt()
        .without_time() // only on local deployments
        .with_target(false)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // get leptos config
    let (leptos_option, addr) = web::routes_leptos::get_leptos_config().await?;

    // region:        --- Axum router

    let routes_all = Router::new()
        .merge(web::routes_leptos::routes(leptos_option))
        .merge(web::routes_api::routes());

    // endregion:     --- Axum router

    // region:        --- Start server

    // Ok to `unwrap` errors here
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("{:<12} - {:?}\n", "LISTENING", listener.local_addr());
    println!("HELO");
    axum::serve(listener, routes_all.into_make_service())
        .await
        .unwrap();

    // endregion:     --- Start server

    Ok(())
}
