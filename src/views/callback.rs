use crate::server_fns;
use crate::state::{self, SessionState};
use dioxus::prelude::*;

#[component]
pub fn OAuthCallback() -> Element {
    let mut status = use_signal(|| "Processing login...".to_string());
    let mut done = use_signal(|| false);
    let mut error = use_signal(|| String::new());

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let params = extract_query_params();
            if let (Some(code), Some(state)) = (params.get("code"), params.get("state")) {
                let code = code.clone();
                let state = state.clone();
                spawn(async move {
                    match server_fns::complete_oauth_server(code, state).await {
                        Ok(session_data) => {
                            let mut session_ctx: Signal<SessionState> = use_context();
                            session_ctx.write().did = session_data.did.clone();
                            session_ctx.write().handle = session_data.handle.clone();
                            session_ctx.write().pds_endpoint =
                                session_data.pds_endpoint.clone();
                            session_ctx.write().access_token =
                                session_data.access_token.clone();
                            session_ctx.write().is_authenticated = true;

                            #[cfg(target_arch = "wasm32")]
                            {
                                let state = session_ctx.read().clone();
                                state::save_session(&state);
                            }

                            status.set("Login successful! Redirecting...".to_string());
                            done.set(true);

                            let nav = navigator();
                            nav.push(crate::Route::Home {});
                        }
                        Err(e) => {
                            error.set(format!("Login failed: {}", e));
                        }
                    }
                });
            } else {
                error.set("Missing code or state parameter in callback URL".to_string());
            }
        }
    });

    if !error.read().is_empty() {
        return rsx! {
            div { class: "flex flex-col items-center justify-center min-h-screen p-4",
                div { class: "max-w-md text-center",
                    h1 { class: "text-3xl font-bold mb-4 text-ctp-red",
                        "Login Failed"
                    }
                    p { class: "text-ctp-subtext0 mb-6",
                        "{error}"
                    }
                    Link {
                        to: crate::Route::Home {},
                        class: "px-6 py-3 bg-ctp-mauve text-ctp-base rounded-lg hover:bg-ctp-pink transition-colors",
                        "Try Again"
                    }
                }
            }
        };
    }

    rsx! {
        div { class: "flex flex-col items-center justify-center min-h-screen p-4",
            div { class: "text-center",
                h1 { class: "text-3xl font-bold mb-4 text-ctp-text",
                    if done() { "Welcome!" } else { "Logging in..." }
                }
                p { class: "text-ctp-subtext0",
                    "{status}"
                }
                if !done() {
                    div { class: "mt-4 w-8 h-8 border-4 border-ctp-mauve border-t-transparent rounded-full animate-spin mx-auto" }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn extract_query_params() -> std::collections::HashMap<String, String> {
    let window = web_sys::window().expect("no window");
    let location = window.location();
    let search = location.search().unwrap_or_default();
    if search.is_empty() {
        return std::collections::HashMap::new();
    }
    let query = search.trim_start_matches('?');
    url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect()
}
