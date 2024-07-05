use leptos::{component, view, IntoView};
use leptos_router::A;

#[component]
pub fn Page404() -> impl IntoView {
    view! {
        <div>
            <h1 class="text-4xl font-serif my-5">404 not found</h1>
            <A class="mt-5 rounded-md p-3 h-8 bg-lime-300 hover:bg-lime-100" href="/">
                To home page
            </A>
        </div>
    }
}
