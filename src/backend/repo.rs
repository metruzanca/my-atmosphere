use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

use atproto_identity::url::build_url;
use atproto_oauth::dpop::request_dpop;
use serde::Deserialize;

use super::oauth::{SessionData, ACTIVE_SESSIONS};
use crate::types::DiscoveredApp;

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

    let mut apps: Vec<DiscoveredApp> = prefix_map
        .into_iter()
        .map(|(prefix, collections)| {
            let record_count = collections.len();
            DiscoveredApp {
                display_name: nsid_prefix_to_name(&prefix),
                url: nsid_prefix_to_url(&prefix),
                icon: pick_icon(&prefix),
                color: pick_color(&prefix),
                nsid_prefix: prefix,
                record_count,
                collections,
            }
        })
        .collect();

    apps.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));

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

fn nsid_prefix_to_name(prefix: &str) -> String {
    prefix
        .split('.')
        .rev()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn nsid_prefix_to_url(prefix: &str) -> String {
    let parts: Vec<&str> = prefix.split('.').collect();
    let reversed: Vec<&str> = parts.into_iter().rev().collect();
    format!("https://{}", reversed.join("."))
}

fn hash_prefix(prefix: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    prefix.hash(&mut hasher);
    hasher.finish()
}

fn pick_icon(prefix: &str) -> String {
    const ICONS: &[&str] = &[
        "📦", "🔧", "📡", "💡", "🎯", "🚀", "⭐", "🔮",
        "📋", "🎨", "🔌", "📊", "🌱", "⚡", "🛠", "📚",
    ];
    ICONS[hash_prefix(prefix) as usize % ICONS.len()].to_string()
}

fn pick_color(prefix: &str) -> String {
    const COLORS: &[&str] = &[
        "#cba6f7", "#f5c2e7", "#89b4fa", "#a6e3a1",
        "#fab387", "#94e2d5", "#74c7ec", "#f38ba8",
        "#b4befe", "#f9e2af", "#eba0ac", "#89dceb",
    ];
    COLORS[hash_prefix(prefix) as usize % COLORS.len()].to_string()
}
