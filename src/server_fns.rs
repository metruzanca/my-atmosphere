use dioxus::prelude::*;

use crate::types::DiscoveredApp;
use atproto_oauth_dioxus::types::SessionData;

#[server]
pub async fn scan_apps_server(session: SessionData) -> Result<Vec<DiscoveredApp>, ServerFnError> {
    let backend_session = atproto_oauth_dioxus::types::SessionData {
        did: session.did,
        handle: session.handle,
        pds_endpoint: session.pds_endpoint,
        access_token: session.access_token,
    };
    crate::backend::repo::scan_apps(&backend_session)
        .await
        .map(|apps| {
            apps.into_iter()
                .map(|a| DiscoveredApp {
                    nsid_prefix: a.nsid_prefix,
                    display_name: a.display_name,
                    icon: a.icon,
                    color: a.color,
                    url: a.url,
                    record_count: a.record_count,
                    collections: a.collections,
                })
                .collect()
        })
        .map_err(ServerFnError::new)
}
