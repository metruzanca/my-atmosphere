use atproto_oauth_dioxus::hooks::do_atproto_login;
use dioxus::prelude::*;

#[component]
pub fn Login() -> Element {
    let mut handle = use_signal(String::new);
    let auth_url = use_signal(|| None::<String>);
    let error = use_signal(|| None::<String>);
    let is_loading = use_signal(|| false);

    if let Some(url) = auth_url.read().as_ref() {
        return rsx! {
            div { class: "flex flex-col items-center justify-center min-h-screen p-4",
                h1 { class: "text-3xl font-bold mb-6 text-center text-ctp-text",
                    "Redirecting to your AT Protocol provider..."
                }
                p { class: "text-ctp-subtext0 mb-4 text-center max-w-md",
                    "You'll be redirected to authorize my-atmosphere to view the apps you use on AT Protocol."
                }
                a {
                    href: "{url}",
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
                    "Your AT Protocol app dashboard"
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
                                do_atproto_login(handle(), auth_url, error, is_loading);
                            }
                        },
                        disabled: is_loading(),
                    }

                    button {
                        class: "w-full px-6 py-3 bg-ctp-mauve text-ctp-base rounded-lg hover:bg-ctp-pink transition-colors disabled:opacity-50 disabled:cursor-not-allowed font-medium cursor-pointer",
                        disabled: is_loading() || handle.read().is_empty(),
                        onclick: move |_| do_atproto_login(handle(), auth_url, error, is_loading),
                        if is_loading() {
                            "Connecting..."
                        } else {
                            "Login with AT Protocol"
                        }
                    }

                    if let Some(err) = error.read().as_ref() {
                        div { class: "p-3 bg-ctp-surface0 border border-ctp-red rounded-lg text-ctp-red text-sm",
                            "{err}"
                        }
                    }
                }
            }
        }
    }
}
