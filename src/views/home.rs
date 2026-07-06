#[cfg_attr(not(target_arch = "wasm32"), allow(unused_imports))]
use crate::state::{self, SessionState};
use crate::views::{Dashboard, Login};
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    #[allow(unused_mut)]
    let mut session = use_context::<Signal<SessionState>>();

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            if !session.read().is_authenticated {
                if let Some(stored) = state::load_session() {
                    session.set(stored);
                }
            }
        }
    });

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
                                #[cfg(target_arch = "wasm32")]
                                state::clear_session();
                                let mut s = use_context::<Signal<SessionState>>();
                                *s.write() = SessionState::default();
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
