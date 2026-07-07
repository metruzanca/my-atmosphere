use atproto_oauth_dioxus::hooks::do_atproto_logout;
use atproto_oauth_dioxus::types::SessionState;
use crate::views::{Dashboard, Login};
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let session = use_context::<Signal<SessionState>>();

    let is_authenticated = session.read().is_authenticated;
    let handle = session.read().handle.clone();
    let mut logged_out = use_signal(|| false);

    rsx! {
        div { class: "min-h-screen flex flex-col",
            if is_authenticated && !logged_out() {
                div { class: "flex items-center justify-between px-6 py-3 border-b border-ctp-surface0",
                    span { class: "text-lg font-semibold text-ctp-text",
                        "my-atmosphere"
                    }
                    div { class: "flex items-center gap-3",
                        span { class: "text-sm text-ctp-subtext0",
                            "{handle}"
                        }
                        button {
                            class: "px-3 py-1.5 text-sm rounded-lg bg-ctp-surface0 text-ctp-subtext1 hover:bg-ctp-surface1 hover:text-ctp-text transition-colors cursor-pointer",
                            onclick: move |_| {
                                do_atproto_logout(session);
                                logged_out.set(true);
                            },
                            "Log out"
                        }
                    }
                }
                Dashboard {}
            } else {
                Login {}
            }
        }
    }
}
