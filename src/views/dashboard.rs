use crate::server_fns;
use crate::state::SessionState;
use crate::types::DiscoveredApp;
use dioxus::prelude::*;

#[component]
pub fn Dashboard() -> Element {
    let session = use_context::<Signal<SessionState>>();

    let apps: Resource<Result<Vec<DiscoveredApp>, String>> = use_resource(move || {
        let session = session.read().clone();
        async move {
            let s = crate::types::SessionData {
                did: session.did,
                handle: session.handle,
                pds_endpoint: session.pds_endpoint,
                access_token: session.access_token,
            };
            server_fns::scan_apps_server(s)
                .await
                .map_err(|e| e.to_string())
        }
    });

    let _handle = session.read().handle.clone();

    rsx! {
        div { class: "flex-1 p-6",
            div { class: "max-w-7xl mx-auto",
                div { class: "mb-8",
                    h1 { class: "text-3xl font-bold text-ctp-text mb-2",
                        "Your AT Protocol Apps"
                    }
                }

                match apps() {
                    Some(Ok(app_list)) => {
                        if app_list.is_empty() {
                            rsx! {
                                div { class: "text-center py-16",
                                    p { class: "text-ctp-subtext0 text-lg",
                                        "No apps found in your repository yet."
                                    }
                                }
                            }
                        } else {
                            rsx! {
                                div { class: "masonry",
                                    for app in app_list {
                                        div { class: "masonry-item",
                                            AppCard {
                                                app: app.clone(),
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Some(Err(e)) => {
                        rsx! {
                            div { class: "p-4 bg-ctp-surface0 border border-ctp-red rounded-lg text-ctp-red",
                                "Failed to scan apps: {e}"
                            }
                        }
                    }
                    None => {
                        rsx! {
                            div { class: "flex flex-col items-center justify-center py-16",
                                div { class: "w-12 h-12 border-4 border-ctp-mauve border-t-transparent rounded-full animate-spin mb-4" }
                                p { class: "text-ctp-subtext0",
                                    "Loading your apps..."
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn AppCard(app: DiscoveredApp) -> Element {
    let color = app.color.clone();
    let has_url = !app.url.is_empty();

    rsx! {
        div {
            class: "bg-ctp-surface0 rounded-xl overflow-hidden shadow-lg hover:shadow-xl transition-shadow group",
            border_color: "{color}",
            border_left_width: "4px",
            border_left_style: "solid",

            div { class: "p-5",
                div { class: "flex items-start justify-between mb-3",
                    if has_url {
                        a {
                            href: "{app.url}",
                            target: "_blank",
                            class: "text-3xl hover:scale-110 transition-transform inline-block",
                            "{app.icon}"
                        }
                    } else {
                        span { class: "text-3xl",
                            "{app.icon}"
                        }
                    }
                    span { class: "px-2 py-1 bg-ctp-surface1 rounded-full text-xs text-ctp-subtext0",
                        "{app.record_count} records"
                    }
                }

                if has_url {
                    a {
                        href: "{app.url}",
                        target: "_blank",
                        class: "text-lg font-semibold text-ctp-text mb-1 group-hover:text-ctp-mauve transition-colors hover:underline inline-block",
                        "{app.display_name}"
                    }
                } else {
                    h3 { class: "text-lg font-semibold text-ctp-text mb-1 group-hover:text-ctp-mauve transition-colors",
                        "{app.display_name}"
                    }
                }

                div { class: "pt-3 border-t border-ctp-surface1",
                    span { class: "text-xs text-ctp-overlay0 font-mono bg-ctp-mantle px-2 py-1 rounded",
                        "{app.nsid_prefix}.*"
                    }
                }

                if !app.collections.is_empty() {
                    div { class: "mt-3 space-y-1",
                        for collection in app.collections.iter().take(3) {
                            div { class: "text-xs text-ctp-overlay0 font-mono truncate",
                                "{collection}"
                            }
                        }
                        if app.collections.len() > 3 {
                            div { class: "text-xs text-ctp-overlay1",
                                "+{app.collections.len() - 3} more..."
                            }
                        }
                    }
                }
            }
        }
    }
}
