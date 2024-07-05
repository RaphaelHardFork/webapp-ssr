mod components;
mod error;
mod pages;

pub use error::{Error, Result};

use leptos::{component, view, IntoView};
use leptos_meta::{provide_meta_context, Link, Stylesheet, Title};
use leptos_router::{Route, Router, Routes};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/start-axum-workspace.css"/>
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Title text="Client intranet"/>
        <Router fallback=|| pages::Page404.into_view()>
            <main class="bg-gradient-to-tr from-blue-100 to-blue-50 min-h-screen p-7">
                <Routes>
                    <Route path="/" view=pages::Login/>
                    <Route path="/error" view=pages::Error/>
                </Routes>
            </main>
        </Router>
    }
}
