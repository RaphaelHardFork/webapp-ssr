mod web;

use axum::Router;
use leptos::get_configuration;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // create a global subscriber
    tracing_subscriber::fmt()
        .without_time() // only on local deployments
        .with_target(false)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // FIXME: change the way config is get
    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    let config = get_configuration(None).await.unwrap();
    let leptos_options = &config.leptos_options;
    let addr = leptos_options.site_addr;

    // build our application with a route
    let routes_all = Router::new()
        .merge(web::routes_leptos::routes(config))
        .merge(web::routes_api::routes()); // api test

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("{:<12} - {:?}\n", "LISTENING", listener.local_addr());
    println!("HELO");
    axum::serve(listener, routes_all.into_make_service())
        .await
        .unwrap();
}
