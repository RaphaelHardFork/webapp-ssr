use leptos::{component, view, IntoView};

#[component]
pub fn Error() -> impl IntoView {
    view! {
        <div>
            <h1>Error, please try again later</h1>
        </div>
    }
}
