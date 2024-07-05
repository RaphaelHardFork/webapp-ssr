use crate::components::ErrorAlert;
use crate::Error;
use leptos::{component, create_signal, event_target_value, view, IntoView, SignalGet, SignalSet};
use leptos_router::Form;

#[component]
pub fn LoginForm() -> impl IntoView {
    let (error, set_error) = create_signal::<Option<Error>>(None);
    let (email, set_email) = create_signal::<String>(String::new());
    let (pwd, set_pwd) = create_signal::<String>(String::new());

    view! {
        <div class="font-serif mx-auto bg-gray-300 rounded-md shadow-md w-2/4 p-3">
            <ErrorAlert error=error/>
            <Form action="" class="flex flex-col">
                <div class="flex flex-col mb-3">
                    <label class="mb-2" for="email-input">
                        Email:
                    </label>
                    <input
                        class="bg-white rounded-md h-8 p-2"
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
                        if !email.get().is_empty() && !pwd.get().is_empty() {
                            "mt-5 rounded-md h-8 bg-lime-300 hover:bg-lime-100"
                        } else {
                            "mt-5 rounded-md h-8 bg-gray-100"
                        }
                    }

                    // send info to SQLite
                    on:click=move |_| { set_error.set(Some(Error::Unauthorized)) }
                    disabled=move || email.get().is_empty() || pwd.get().is_empty()
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
