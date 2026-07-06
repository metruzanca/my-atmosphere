use std::collections::HashMap;

use atproto_identity::url::build_url;
use atproto_oauth::dpop::request_dpop;
use serde::{Deserialize, Serialize};

use super::oauth::{SessionData, ACTIVE_SESSIONS};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredApp {
    pub nsid_prefix: String,
    pub display_name: String,
    pub description: String,
    pub icon: String,
    pub color: String,
    pub record_count: usize,
    pub collections: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DescribeRepoResponse {
    collections: Vec<String>,
}

pub async fn scan_apps(session: &SessionData) -> Result<Vec<DiscoveredApp>, String> {
    let http_client = reqwest::Client::new();

    let url = build_url(
        &session.pds_endpoint,
        "/xrpc/com.atproto.repo.describeRepo",
        [("repo", session.did.as_str())],
    )
    .map_err(|e| format!("URL build failed: {}", e))?
    .to_string();

    let active = {
        let sessions = ACTIVE_SESSIONS.lock().await;
        sessions
            .get(&session.did)
            .cloned()
            .ok_or("No active session found")?
    };

    let (dpop_token, _, _) =
        request_dpop(&active.dpop_key, "GET", &url, &active.access_token)
            .map_err(|e| format!("DPoP proof failed: {}", e))?;

    let resp = http_client
        .get(&url)
        .header("Authorization", format!("DPoP {}", active.access_token))
        .header("DPoP", &dpop_token)
        .send()
        .await
        .map_err(|e| format!("describeRepo request failed: {}", e))?;

    let repo_data: DescribeRepoResponse = resp
        .json()
        .await
        .map_err(|e| format!("describeRepo parse failed: {}", e))?;

    let mut prefix_map: HashMap<String, Vec<String>> = HashMap::new();
    for collection in &repo_data.collections {
        let prefix = extract_nsid_prefix(collection);
        prefix_map
            .entry(prefix)
            .or_default()
            .push(collection.clone());
    }

    let apps: Vec<DiscoveredApp> = prefix_map
        .into_iter()
        .map(|(prefix, collections)| {
            let record_count = collections.len();
            let meta = APP_REGISTRY.get(prefix.as_str());
            DiscoveredApp {
                nsid_prefix: prefix.clone(),
                display_name: meta
                    .map(|m| m.name)
                    .unwrap_or(&prefix)
                    .to_string(),
                description: meta
                    .map(|m| m.description)
                    .unwrap_or("Unknown application")
                    .to_string(),
                icon: meta.map(|m| m.icon).unwrap_or("📦").to_string(),
                color: meta
                    .map(|m| m.color)
                    .unwrap_or("#6b7280")
                    .to_string(),
                record_count,
                collections,
            }
        })
        .collect();

    Ok(apps)
}

fn extract_nsid_prefix(nsid: &str) -> String {
    let parts: Vec<&str> = nsid.split('.').collect();
    if parts.len() >= 2 {
        format!("{}.{}", parts[0], parts[1])
    } else {
        nsid.to_string()
    }
}

struct AppMeta {
    name: &'static str,
    description: &'static str,
    icon: &'static str,
    color: &'static str,
}

static APP_REGISTRY: std::sync::LazyLock<HashMap<&'static str, AppMeta>> =
    std::sync::LazyLock::new(|| {
        let mut m = HashMap::new();
        m.insert(
            "app.bsky",
            AppMeta {
                name: "Bluesky",
                description: "Social networking on the AT Protocol",
                icon: "🦋",
                color: "#1185fe",
            },
        );
        m.insert(
            "chat.bsky",
            AppMeta {
                name: "Bluesky Chat",
                description: "Direct messaging on Bluesky",
                icon: "💬",
                color: "#1185fe",
            },
        );
        m.insert(
            "sh.tangled",
            AppMeta {
                name: "Tangled",
                description: "Git collaboration on AT Protocol",
                icon: "🔀",
                color: "#6366f1",
            },
        );
        m.insert(
            "dev.keytrace",
            AppMeta {
                name: "Keytrace",
                description: "Cryptographic key verification",
                icon: "🔑",
                color: "#f59e0b",
            },
        );
        m.insert(
            "fyi.atstore",
            AppMeta {
                name: "AT Store",
                description: "App directory and reviews",
                icon: "🏪",
                color: "#10b981",
            },
        );
        m.insert(
            "pub.leaflet",
            AppMeta {
                name: "Leaflet",
                description: "Publishing on AT Protocol",
                icon: "📰",
                color: "#8b5cf6",
            },
        );
        m.insert(
            "site.standard",
            AppMeta {
                name: "Standard Site",
                description: "Personal websites on AT Protocol",
                icon: "🌐",
                color: "#ec4899",
            },
        );
        m.insert(
            "social.popfeed",
            AppMeta {
                name: "Popfeed",
                description: "Social feed and reviews",
                icon: "🔥",
                color: "#ef4444",
            },
        );
        m.insert(
            "community.lexicon",
            AppMeta {
                name: "Lexicon Community",
                description: "Community events and calendars",
                icon: "📅",
                color: "#14b8a6",
            },
        );
        m.insert(
            "com.imlunahey",
            AppMeta {
                name: "LunaHey",
                description: "Guestbooks and leaderboards",
                icon: "📝",
                color: "#a855f7",
            },
        );
        m
    });
