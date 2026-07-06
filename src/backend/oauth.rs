use std::collections::HashMap;
use std::sync::LazyLock;

use atproto_identity::key::{KeyData, KeyType, generate_key};
use atproto_identity::resolve::{HickoryDnsResolver, resolve_subject};
use atproto_oauth::resources::{pds_resources, AuthorizationServer};
use atproto_oauth::workflow::{
    OAuthClient, OAuthRequest, OAuthRequestState, oauth_complete, oauth_init,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

static OAUTH_STATES: LazyLock<Mutex<HashMap<String, StoredOAuthState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) static ACTIVE_SESSIONS: LazyLock<Mutex<HashMap<String, ActiveSession>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone)]
struct StoredOAuthState {
    oauth_request: OAuthRequest,
    auth_server: AuthorizationServer,
    pds_url: String,
    client_id: String,
    redirect_uri: String,
    signing_key: KeyData,
    dpop_key: KeyData,
    handle: String,
}

#[derive(Clone)]
pub(crate) struct ActiveSession {
    pub did: String,
    pub handle: String,
    pub pds_endpoint: String,
    pub access_token: String,
    pub dpop_key: KeyData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthInitResponse {
    pub authorization_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub did: String,
    pub handle: String,
    pub pds_endpoint: String,
    pub access_token: String,
}

fn generate_random_hex(len: usize) -> String {
    let bytes: Vec<u8> = (0..len).map(|_| rand::random::<u8>()).collect();
    hex::encode(bytes)
}

fn urlencoding(s: &str) -> String {
    form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

pub async fn init_oauth(handle: String) -> Result<OAuthInitResponse, String> {
    let http_client = reqwest::Client::new();
    let dns_resolver = HickoryDnsResolver::create_resolver(&[]);
    let did = resolve_subject(&http_client, &dns_resolver, &handle)
        .await
        .map_err(|e| format!("Failed to resolve handle: {}", e))?;

    let pds_url = resolve_did_to_pds(&did).await?;

    let (_protected, auth_server) = pds_resources(&http_client, &pds_url)
        .await
        .map_err(|e| format!("Failed to discover OAuth resources: {}", e))?;

    let redirect_uri = "http://127.0.0.1:8080/oauth/callback".to_string();
    let client_id = format!(
        "http://localhost?redirect_uri={}&scope={}",
        urlencoding(&redirect_uri),
        urlencoding("atproto transition:generic"),
    );

    let (code_verifier, code_challenge) = atproto_oauth::pkce::generate();

    let state = generate_random_hex(16);
    let nonce = generate_random_hex(16);

    let signing_key = generate_key(KeyType::P256Private)
        .map_err(|e| format!("Failed to generate signing key: {}", e))?;
    let dpop_key = generate_key(KeyType::P256Private)
        .map_err(|e| format!("Failed to generate DPoP key: {}", e))?;

    let oauth_client = OAuthClient {
        redirect_uri: redirect_uri.clone(),
        client_id: client_id.clone(),
        private_signing_key_data: signing_key.clone(),
    };

    let oauth_request_state = OAuthRequestState {
        state: state.clone(),
        nonce: nonce.clone(),
        code_challenge,
        scope: "atproto transition:generic".to_string(),
    };

    let par_response = oauth_init(
        &http_client,
        &oauth_client,
        &dpop_key,
        Some(&handle),
        &auth_server,
        &oauth_request_state,
    )
    .await
    .map_err(|e| format!("OAuth init failed: {}", e))?;

    let oauth_request = OAuthRequest {
        oauth_state: state.clone(),
        issuer: auth_server.issuer.clone(),
        authorization_server: auth_server.pushed_authorization_request_endpoint.clone(),
        nonce,
        pkce_verifier: code_verifier,
        signing_public_key: hex::encode(&signing_key.1),
        dpop_private_key: hex::encode(&dpop_key.1),
        created_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now()
            + chrono::TimeDelta::seconds(par_response.expires_in as i64),
    };

    OAUTH_STATES.lock().await.insert(
        state.clone(),
        StoredOAuthState {
            oauth_request,
            auth_server: auth_server.clone(),
            pds_url,
            client_id: client_id.clone(),
            redirect_uri,
            signing_key,
            dpop_key,
            handle: handle.clone(),
        },
    );

    let authorization_url = format!(
        "{}?client_id={}&request_uri={}&state={}",
        auth_server.authorization_endpoint,
        urlencoding(&client_id),
        urlencoding(&par_response.request_uri),
        urlencoding(&state),
    );

    Ok(OAuthInitResponse {
        authorization_url,
    })
}

pub async fn complete_oauth(code: String, state: String) -> Result<SessionData, String> {
    let stored = {
        let mut states = OAUTH_STATES.lock().await;
        states
            .remove(&state)
            .ok_or("Invalid or expired OAuth state")?
    };

    let http_client = reqwest::Client::new();

    let oauth_client = OAuthClient {
        redirect_uri: stored.redirect_uri.clone(),
        client_id: stored.client_id.clone(),
        private_signing_key_data: stored.signing_key.clone(),
    };

    let token_response = oauth_complete(
        &http_client,
        &oauth_client,
        &stored.dpop_key,
        &code,
        &stored.oauth_request,
        &stored.auth_server,
    )
    .await
    .map_err(|e| format!("Token exchange failed: {}", e))?;

    let did = token_response
        .sub
        .ok_or("Token response missing 'sub' (DID) field")?;

    ACTIVE_SESSIONS.lock().await.insert(
        did.clone(),
        ActiveSession {
            did: did.clone(),
            handle: stored.handle.clone(),
            pds_endpoint: stored.pds_url.clone(),
            access_token: token_response.access_token.clone(),
            dpop_key: stored.dpop_key.clone(),
        },
    );

    Ok(SessionData {
        did,
        handle: stored.handle,
        pds_endpoint: stored.pds_url,
        access_token: token_response.access_token,
    })
}

async fn resolve_did_to_pds(did: &str) -> Result<String, String> {
    let http_client = reqwest::Client::new();
    let did_doc_url = format!("https://plc.directory/{}", did);
    let resp = http_client
        .get(&did_doc_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch DID doc: {}", e))?;

    let doc: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse DID doc: {}", e))?;

    for svc in doc["service"]
        .as_array()
        .ok_or("No services in DID document")?
    {
        if svc["id"].as_str() == Some("#atproto_pds") {
            return svc["serviceEndpoint"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or("No serviceEndpoint for atproto_pds".to_string());
        }
    }

    Err("No atproto_pds service found".to_string())
}
