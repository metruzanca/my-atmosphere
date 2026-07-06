use dioxus::prelude::*;

use crate::types::{DiscoveredApp, OAuthInitResponse, SessionData};

#[server]
pub async fn init_oauth_server(handle: String) -> Result<OAuthInitResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        crate::backend::oauth::init_oauth(handle)
            .await
            .map(|r| OAuthInitResponse {
                authorization_url: r.authorization_url,
            })
            .map_err(|e| ServerFnError::new(e))
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = handle;
        unreachable!()
    }
}

#[server]
pub async fn complete_oauth_server(
    code: String,
    state: String,
) -> Result<SessionData, ServerFnError> {
    #[cfg(feature = "server")]
    {
        crate::backend::oauth::complete_oauth(code, state)
            .await
            .map(|s| SessionData {
                did: s.did,
                handle: s.handle,
                pds_endpoint: s.pds_endpoint,
                access_token: s.access_token,
            })
            .map_err(|e| ServerFnError::new(e))
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = (code, state);
        unreachable!()
    }
}

#[server]
pub async fn scan_apps_server(session: SessionData) -> Result<Vec<DiscoveredApp>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let backend_session = crate::backend::oauth::SessionData {
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
                        description: a.description,
                        icon: a.icon,
                        color: a.color,
                        record_count: a.record_count,
                        collections: a.collections,
                    })
                    .collect()
            })
            .map_err(|e| ServerFnError::new(e))
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = session;
        unreachable!()
    }
}
