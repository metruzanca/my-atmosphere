use std::collections::HashMap;
use std::sync::LazyLock;

use atproto_identity::key::{KeyData, KeyType, generate_key};
use atproto_identity::resolve::{HickoryDnsResolver, resolve_subject};
use atproto_oauth::dpop::auth_dpop;
use atproto_oauth::jwt::{mint, Claims, Header, JoseClaims};
use atproto_oauth::pkce;
use atproto_oauth::resources::{pds_resources, AuthorizationServer};
use atproto_oauth::workflow::{OAuthRequest, ParResponse};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

static OAUTH_STATES: LazyLock<Mutex<HashMap<String, StoredOAuthState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) static ACTIVE_SESSIONS: LazyLock<Mutex<HashMap<String, ActiveSession>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone)]
pub(crate) struct ActiveSession {
    pub did: String,
    pub handle: String,
    pub pds_endpoint: String,
    pub access_token: String,
    pub dpop_key: KeyData,
}

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

async fn resolve_handle_to_did(handle: &str) -> Result<String, String> {
    let http_client = reqwest::Client::new();
    let dns_resolver = HickoryDnsResolver::create_resolver(&[]);
    resolve_subject(&http_client, &dns_resolver, handle)
        .await
        .map_err(|e| format!("Failed to resolve handle '{}': {}", handle, e))
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

    for svc in doc["service"].as_array().ok_or("No services in DID document")? {
        if svc["id"].as_str() == Some("#atproto_pds") {
            return svc["serviceEndpoint"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or("No serviceEndpoint for atproto_pds".to_string());
        }
    }

    Err("No atproto_pds service found in DID document".to_string())
}

fn generate_random_hex(len: usize) -> String {
    let bytes: Vec<u8> = (0..len).map(|_| rand::random::<u8>()).collect();
    hex::encode(bytes)
}

fn key_data_to_string(key: &KeyData) -> String {
    hex::encode(&key.1)
}

fn urlencoding(s: &str) -> String {
    form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn build_client_assertion(
    signing_key: &KeyData,
    client_id: &str,
    auth_server: &AuthorizationServer,
) -> Result<String, String> {
    let header: Header = signing_key.clone().try_into().map_err(|e| {
        format!("JWT header creation failed: {}", e)
    })?;

    let jti = generate_random_hex(30);
    let now = chrono::Utc::now().timestamp() as u64;

    let claims = Claims::new(JoseClaims {
        issuer: Some(client_id.to_string()),
        subject: Some(client_id.to_string()),
        audience: Some(auth_server.issuer.clone()),
        json_web_token_id: Some(jti),
        issued_at: Some(now),
        ..Default::default()
    });

    mint(signing_key, &header, &claims)
        .map_err(|e| format!("JWT minting failed: {}", e))
}

pub async fn init_oauth(handle: String) -> Result<OAuthInitResponse, String> {
    let did = resolve_handle_to_did(&handle).await?;
    let pds_url = resolve_did_to_pds(&did).await?;

    let http_client = reqwest::Client::new();
    let (_protected, auth_server) = pds_resources(&http_client, &pds_url)
        .await
        .map_err(|e| format!("Failed to discover OAuth resources: {}", e))?;

    let (code_verifier, code_challenge) = pkce::generate();

    let state = generate_random_hex(16);
    let nonce = generate_random_hex(16);

    let redirect_uri = "http://127.0.0.1:8080/oauth/callback".to_string();
    let client_id = format!(
        "http://localhost?redirect_uri={}&scope={}",
        urlencoding(&redirect_uri),
        urlencoding("atproto transition:generic"),
    );

    let signing_key = generate_key(KeyType::P256Private)
        .map_err(|e| format!("Failed to generate signing key: {}", e))?;
    let dpop_key = generate_key(KeyType::P256Private)
        .map_err(|e| format!("Failed to generate DPoP key: {}", e))?;

    let client_assertion =
        build_client_assertion(&signing_key, &client_id, &auth_server)?;

    let par_url = &auth_server.pushed_authorization_request_endpoint;

    let par_params = [
        ("response_type", "code"),
        ("code_challenge", &code_challenge),
        ("code_challenge_method", "S256"),
        ("client_id", client_id.as_str()),
        ("state", state.as_str()),
        ("redirect_uri", redirect_uri.as_str()),
        ("scope", "atproto transition:generic"),
        ("application_type", "native"),
        (
            "client_assertion_type",
            "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
        ),
        ("client_assertion", &client_assertion),
        ("login_hint", handle.as_str()),
    ];

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ParResult {
        Ok(ParResponse),
        Err {
            error: String,
            #[serde(rename = "error_description")]
            _description: Option<String>,
        },
    }

    let resp = http_client
        .post(par_url)
        .form(&par_params)
        .send()
        .await
        .map_err(|e| format!("PAR request failed: {}", e))?;

    let par_result: ParResult = resp
        .json()
        .await
        .map_err(|e| format!("PAR response parse failed: {}", e))?;

    let par_response = match par_result {
        ParResult::Ok(pr) => pr,
        ParResult::Err { error, .. } => {
            return Err(format!("PAR error: {}", error));
        }
    };

    let oauth_request = OAuthRequest {
        oauth_state: state.clone(),
        issuer: auth_server.issuer.clone(),
        authorization_server: auth_server.pushed_authorization_request_endpoint.clone(),
        nonce,
        pkce_verifier: code_verifier,
        signing_public_key: key_data_to_string(&signing_key),
        dpop_private_key: key_data_to_string(&dpop_key),
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

    let client_assertion = build_client_assertion(
        &stored.signing_key,
        &stored.client_id,
        &stored.auth_server,
    )?;

    let token_params = [
        ("client_id", stored.client_id.as_str()),
        ("redirect_uri", stored.redirect_uri.as_str()),
        ("grant_type", "authorization_code"),
        ("code", code.as_str()),
        ("code_verifier", &stored.oauth_request.pkce_verifier),
        (
            "client_assertion_type",
            "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
        ),
        ("client_assertion", &client_assertion),
    ];

    let token_endpoint = &stored.auth_server.token_endpoint;

    let (dpop_token, dpop_header, mut dpop_claims) =
        auth_dpop(&stored.dpop_key, "POST", token_endpoint)
            .map_err(|e| format!("DPoP proof generation failed: {}", e))?;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum TokenResult {
        Ok {
            access_token: String,
            #[allow(dead_code)]
            token_type: String,
            #[allow(dead_code)]
            scope: String,
            #[allow(dead_code)]
            expires_in: u32,
            sub: Option<String>,
        },
        Err {
            error: String,
            #[serde(rename = "error_description")]
            _description: Option<String>,
        },
    }

    let resp = http_client
        .post(token_endpoint)
        .header("DPoP", &dpop_token)
        .form(&token_params)
        .send()
        .await
        .map_err(|e| format!("Token request failed: {}", e))?;

    let server_nonce = resp
        .headers()
        .get("DPoP-Nonce")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let token_result: TokenResult = resp
        .json()
        .await
        .map_err(|e| format!("Token response parse failed: {}", e))?;

    let (access_token, did) = match token_result {
        TokenResult::Ok { access_token, sub, .. } => {
            let did = sub.ok_or("Token response missing 'sub' (DID) field")?;
            (access_token, did)
        }
        TokenResult::Err { error, .. } if error == "use_dpop_nonce" => {
            let nonce =
                server_nonce.ok_or("Token endpoint requested DPoP nonce but none provided")?;

            dpop_claims.jose.issued_at = Some(chrono::Utc::now().timestamp() as u64);
            dpop_claims.jose.json_web_token_id = Some(generate_random_hex(16));
            dpop_claims.private.insert(
                "nonce".to_string(),
                serde_json::Value::String(nonce),
            );

            let new_dpop_token = mint(&stored.dpop_key, &dpop_header, &dpop_claims)
                .map_err(|e| format!("DPoP re-mint failed: {}", e))?;

            let resp = http_client
                .post(token_endpoint)
                .header("DPoP", &new_dpop_token)
                .form(&token_params)
                .send()
                .await
                .map_err(|e| format!("Token retry request failed: {}", e))?;

            let token_result2: TokenResult = resp
                .json()
                .await
                .map_err(|e| format!("Token retry parse failed: {}", e))?;

            match token_result2 {
                TokenResult::Ok { access_token, sub, .. } => {
                    let did = sub.ok_or("Token response missing 'sub' (DID) field on retry")?;
                    (access_token, did)
                }
                TokenResult::Err { error, .. } => {
                    return Err(format!("Token exchange error: {}", error));
                }
            }
        }
        TokenResult::Err { error, .. } => {
            return Err(format!("Token exchange error: {}", error));
        }
    };

    ACTIVE_SESSIONS.lock().await.insert(
        did.clone(),
        ActiveSession {
            did: did.clone(),
            handle: stored.handle.clone(),
            pds_endpoint: stored.pds_url.clone(),
            access_token: access_token.clone(),
            dpop_key: stored.dpop_key.clone(),
        },
    );

    Ok(SessionData {
        did,
        handle: stored.handle,
        pds_endpoint: stored.pds_url,
        access_token,
    })
}
