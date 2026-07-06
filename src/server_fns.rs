use dioxus::prelude::*;

use crate::types::{ClientMetadata, DiscoveredApp, OAuthInitResponse, SessionData};

#[cfg(feature = "server")]
use atproto_identity::key::to_public;
#[cfg(feature = "server")]
use atproto_oauth::jwk::{generate, WrappedJsonWebKeySet};

#[server]
pub async fn init_oauth_server(handle: String) -> Result<OAuthInitResponse, ServerFnError> {
    crate::backend::oauth::init_oauth(handle)
        .await
        .map(|r| OAuthInitResponse {
            authorization_url: r.authorization_url,
        })
        .map_err(ServerFnError::new)
}

#[server]
pub async fn complete_oauth_server(
    code: String,
    state: String,
) -> Result<SessionData, ServerFnError> {
    crate::backend::oauth::complete_oauth(code, state)
        .await
        .map(|s| SessionData {
            did: s.did,
            handle: s.handle,
            pds_endpoint: s.pds_endpoint,
            access_token: s.access_token,
        })
        .map_err(ServerFnError::new)
}

#[server]
pub async fn scan_apps_server(session: SessionData) -> Result<Vec<DiscoveredApp>, ServerFnError> {
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

#[get("/oauth/client-metadata.json")]
pub async fn client_metadata_server() -> Result<ClientMetadata, ServerFnError> {
    let base = crate::backend::oauth::base_url();
    let signing_key = crate::backend::oauth::get_signing_key();
    let public_key = to_public(signing_key)
        .map_err(|e| ServerFnError::new(format!("Failed to derive public key: {}", e)))?;
    let jwk = generate(&public_key)
        .map_err(|e| ServerFnError::new(format!("Failed to generate JWK: {}", e)))?;
    let jwks = serde_json::to_value(WrappedJsonWebKeySet { keys: vec![jwk] })
        .map_err(|e| ServerFnError::new(format!("Failed to serialize JWKS: {}", e)))?;

    Ok(ClientMetadata {
        client_id: format!("{}/oauth/client-metadata.json", base),
        dpop_bound_access_tokens: true,
        application_type: "web".to_string(),
        redirect_uris: vec![format!("{}/oauth/callback", base)],
        grant_types: vec![
            "authorization_code".to_string(),
            "refresh_token".to_string(),
        ],
        response_types: vec!["code".to_string()],
        scope: "atproto transition:generic".to_string(),
        token_endpoint_auth_method: "private_key_jwt".to_string(),
        subject_type: "public".to_string(),
        token_endpoint_auth_signing_alg: "ES256".to_string(),
        jwks,
    })
}
