use crate::components::ErrorAlert;
use crate::utils::validate_email;
use crate::Error;
use leptos::logging::log;
use leptos::{
    component, create_action, create_effect, create_resource, create_signal, event_target_value,
    expect_context, Suspense,
};
use leptos::{server, spawn_local, ServerFnError};
use leptos::{view, IntoView, Show, SignalGet, SignalSet};
use leptos_router::Form;
use web_sys::MouseEvent;

#[server]
async fn add_user(email: String, pwd: String) -> Result<Option<i64>, ServerFnError> {
    use lib_core::model::app_state::AppState;
    use lib_core::model::user::create_user;

    let app_state: AppState = expect_context();

    let id = create_user(app_state.mm.clone(), &email, &pwd).await?;

    Ok(id)
}

#[component]
pub fn LoginForm() -> impl IntoView {
    // define signals (states)
    let (error, set_error) = create_signal::<Option<Error>>(None);
    let (email, set_email) = create_signal::<String>(String::new());
    let (pwd, set_pwd) = create_signal::<String>(String::new());

    // derived signals
    let empty_email = move || email.get().is_empty();
    let empty_pwd = move || pwd.get().is_empty();
    let valid_email = move || validate_email(&email.get());

    // region:        --- Login action

    // create action
    let login_action = create_action(|input: &(String, String)| {
        let (email, pwd) = input.clone();
        async move { add_user(email, pwd).await }
    });

    // trigger action
    let handle_login = move |_: MouseEvent| {
        spawn_local(async move { login_action.dispatch((email.get(), pwd.get())) })
    };

    // react to action response
    create_effect(move |_| {
        if let Some(res) = login_action.value().get() {
            match res {
                Ok(id) => {
                    if let Some(id) = id {
                        set_error.set(Some(Error::ServerError { code: id }))
                    } else {
                        set_error.set(Some(Error::TryLater))
                    }
                }
                Err(_) => set_error.set(Some(Error::TryLater)),
            }
        }
    });

    // endregion:     --- Login action

    view! {
        <div class="font-serif mx-auto bg-gray-300 rounded-md shadow-md w-2/4 p-3">
            <Show
                when=move || !login_action.pending().get()
                fallback=|| {
                    view! {
                        <div class="bg-yellow-200 p-2 text-center rounded-md mb-4">Loading...</div>
                    }
                }
            >

                <ErrorAlert error=error/>
            </Show>

            <Form action="" class=" flex flex-col">
                <div class="flex flex-col mb-3">
                    <label class="mb-2" for="email-input">
                        Email:
                    </label>
                    <input
                        class=move || match valid_email() {
                            true => "bg-white rounded-md h-8 p-2",
                            false => "bg-white rounded-md h-8 p-2 border-2 border-red-400",
                        }

                        type="email"
                        placeholder="e@mail.com"
                        id="email-input"
                        value=(move || email.get())()
                        on:input=move |ev| { set_email.set(event_target_value(&ev)) }
                        prop:value=email
                    />
                </div>
                <div class="flex flex-col mb-3">
                    <label class="mb-2" for="pwd-input">
                        Password:
                    </label>
                    <input
                        class="bg-white rounded-md h-8 p-2"
                        type="password"
                        placeholder="*************"
                        id="pwd-input"
                        value=(move || pwd.get())()
                        on:input=move |ev| { set_pwd.set(event_target_value(&ev)) }
                        prop:value=pwd
                    />
                </div>
                <button
                    class=move || {
                        if !empty_email() && !empty_pwd() && valid_email() {
                            "mt-5 rounded-md h-8 bg-lime-300 hover:bg-lime-100"
                        } else {
                            "mt-5 rounded-md h-8 bg-gray-100"
                        }
                    }

                    // send info to SQLite
                    on:click=handle_login
                    disabled=move || { empty_email() || empty_pwd() || !valid_email() }
                >

                    Sign in
                </button>
            </Form>

        </div>
    }
}

// region:    --- Tests

#[cfg(test)]
mod tests {
    type Error = Box<dyn std::error::Error>;
    type Result<T> = core::result::Result<T, Error>; // For tests.

    use leptos::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    use super::LoginForm;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn create() -> Result<()> {
        // test section
        let document = leptos::document();
        let test_wrapper = document
            .create_element("section")
            .expect("Cannot create document");
        let _ = document.body().unwrap().append_child(&test_wrapper);

        // mount into the DOM
        mount_to(
            test_wrapper.clone().unchecked_into(),
            || view! { <LoginForm/> },
        );

        // extract inputs
        let input = test_wrapper
            .query_selector("#email-input")
            .unwrap()
            .unwrap()
            .unchecked_into::<web_sys::HtmlInputElement>();

        assert_eq!(
            input.placeholder(),
            "Email".to_string(),
            "email placeholder"
        );

        // extract the element
        if let Some(btn_text) = test_wrapper
            .query_selector("button")
            .unwrap()
            .unwrap()
            .text_content()
        {
            assert_eq!(btn_text.trim(), "Sign in".to_string());
        }

        // runtime
        // let runtime = create_runtime();
        // runtime.dispose();

        // clean the browser
        test_wrapper.remove();
        Ok(())
    }

    #[test]
    fn test_logic() -> Result<()> {
        assert_eq!(1, 1, "Whow");
        Ok(())
    }
}

// endregion: --- Tests
