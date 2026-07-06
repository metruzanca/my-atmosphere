use crate::server_fns;
use dioxus::prelude::*;

fn do_login(
    handle: Signal<String>,
    mut loading: Signal<bool>,
    mut error: Signal<String>,
    mut auth_url: Signal<String>,
) {
    let h = handle.read().clone();
    if h.is_empty() {
        return;
    }
    loading.set(true);
    error.set(String::new());
    spawn(async move {
        match server_fns::init_oauth_server(h).await {
            Ok(resp) => {
                auth_url.set(resp.authorization_url);
                loading.set(false);
            }
            Err(e) => {
                error.set(format!("Login failed: {}", e));
                loading.set(false);
            }
        }
    });
}

#[component]
pub fn Login() -> Element {
    let mut handle = use_signal(String::new);
    let loading = use_signal(|| false);
    let error = use_signal(String::new);
    let auth_url = use_signal(String::new);

    if !auth_url.read().is_empty() {
        return rsx! {
            div { class: "flex flex-col items-center justify-center min-h-screen p-4",
                h1 { class: "text-3xl font-bold mb-6 text-center text-ctp-text",
                    "Redirecting to your AT Protocol provider..."
                }
                p { class: "text-ctp-subtext0 mb-4 text-center max-w-md",
                    "You'll be redirected to authorize my-atmosphere to view the apps you use on AT Protocol."
                }
                a {
                    href: "{auth_url}",
                    class: "px-8 py-3 bg-ctp-mauve text-ctp-base rounded-lg hover:bg-ctp-pink transition-colors font-medium text-lg",
                    "Continue to Authorization"
                }
            }
        };
    }

    rsx! {
        div { class: "flex flex-col items-center justify-center min-h-screen p-4",
            div { class: "w-full max-w-md",
                h1 { class: "text-4xl font-bold mb-2 text-center text-ctp-text",
                    "my-atmosphere"
                }
                p { class: "text-ctp-subtext0 mb-8 text-center",
                    "Discover the AT Protocol apps you use"
                }

                div { class: "space-y-4",
                    label { class: "block text-sm font-medium text-ctp-subtext1 mb-1",
                        "Enter your AT Protocol handle"
                    }
                    input {
                        class: "w-full px-4 py-3 bg-ctp-surface0 border border-ctp-surface1 rounded-lg text-ctp-text placeholder-ctp-overlay0 focus:outline-none focus:ring-2 focus:ring-ctp-mauve focus:border-transparent",
                        r#type: "text",
                        placeholder: "you.bsky.social",
                        value: "{handle}",
                        oninput: move |e| handle.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                do_login(handle, loading, error, auth_url);
                            }
                        },
                        disabled: loading(),
                    }

                    button {
                        class: "w-full px-6 py-3 bg-ctp-mauve text-ctp-base rounded-lg hover:bg-ctp-pink transition-colors disabled:opacity-50 disabled:cursor-not-allowed font-medium cursor-pointer",
                        disabled: loading() || handle.read().is_empty(),
                        onclick: move |_| do_login(handle, loading, error, auth_url),
                        if loading() {
                            "Connecting..."
                        } else {
                            "Login with AT Protocol"
                        }
                    }

                    if !error.read().is_empty() {
                        div { class: "p-3 bg-ctp-surface0 border border-ctp-red rounded-lg text-ctp-red text-sm",
                            "{error}"
                        }
                    }
                }
            }
        }
    }
}
