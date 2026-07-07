use serde::{Deserialize, Serialize};

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
