use crate::Error;
use leptos::{component, view, IntoView, ReadSignal, SignalGet};

#[component]
pub fn ErrorAlert(error: ReadSignal<Option<Error>>) -> impl IntoView {
    move || {
        if let Some(error) = error.get() {
            let error = format!("Reason: {:?}", error);
            view! { <div class="bg-red-200 p-2 text-center rounded-md mb-4">{error}</div> }
        } else {
            // TODO: change this to avoid a blank space
            view! { <div></div> }
        }
    }
}
