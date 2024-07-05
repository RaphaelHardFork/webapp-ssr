use app::*;
use axum::Router;
use fileserv::file_and_error_handler;
use leptos::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use tracing::info;
use tracing_subscriber::EnvFilter;

pub mod fileserv;

#[tokio::main]
async fn main() {
    // create a global subscriber
    tracing_subscriber::fmt()
        .without_time() // only on local deployments
        .with_target(false)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    // build our application with a route
    let app = Router::new()
        .leptos_routes(&leptos_options, routes, App)
        .fallback(file_and_error_handler)
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("{:<12} - {:?}\n", "LISTENING", listener.local_addr());
    println!("HELO");
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
