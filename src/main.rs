use dioxus::prelude::*;

use atproto_oauth_dioxus::components::AtprotoOAuthCallback as OAuthCallback;
use atproto_oauth_dioxus::components::AtprotoOAuthProvider;
use atproto_oauth_dioxus::config::AtprotoOAuthConfig;

use views::Home;

mod components;
mod views;

mod types;
mod server_fns;

#[cfg(feature = "server")]
mod backend;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[route("/")]
    Home {},
    #[route("/oauth/callback")]
    OAuthCallback {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        AtprotoOAuthProvider {
            config: AtprotoOAuthConfig::new("/oauth/callback"),
            Router::<Route> {}
        }
    }
}
