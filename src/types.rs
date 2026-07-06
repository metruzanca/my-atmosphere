use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscoveredApp {
    pub nsid_prefix: String,
    pub display_name: String,
    pub icon: String,
    pub color: String,
    pub url: String,
    pub record_count: usize,
    pub collections: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientMetadata {
    pub client_id: String,
    pub dpop_bound_access_tokens: bool,
    pub application_type: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub response_types: Vec<String>,
    pub scope: String,
    pub token_endpoint_auth_method: String,
    pub subject_type: String,
    pub token_endpoint_auth_signing_alg: String,
    pub jwks: serde_json::Value,
}
