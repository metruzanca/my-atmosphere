use dioxus::prelude::*;

use views::{Home, OAuthCallback};

mod components;
mod views;

mod state;
use state::SessionState;

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
    use_context_provider(|| Signal::new(SessionState::default()));

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        Router::<Route> {}
    }
}
