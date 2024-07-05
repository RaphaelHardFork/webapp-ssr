use crate::components::LoginForm;
use leptos::{component, view, IntoView};

#[component]
pub fn Login() -> impl IntoView {
    view! {
        <h1 class="text-4xl text-center font-serif my-5">My awesome intranet</h1>
        <LoginForm/>
    }
}
