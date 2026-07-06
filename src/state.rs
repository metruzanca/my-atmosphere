use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SessionState {
    pub is_authenticated: bool,
    pub did: String,
    pub handle: String,
    pub pds_endpoint: String,
    pub access_token: String,
}

#[cfg(target_arch = "wasm32")]
const SESSION_KEY: &str = "my-atmosphere-session";

#[cfg(target_arch = "wasm32")]
pub fn load_session() -> Option<SessionState> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let json = storage.get_item(SESSION_KEY).ok()??;
    serde_json::from_str(&json).ok()
}

#[cfg(target_arch = "wasm32")]
pub fn save_session(state: &SessionState) {
    if let (Some(storage), Ok(json)) = (
        web_sys::window()
            .and_then(|w| w.local_storage().ok())
            .flatten(),
        serde_json::to_string(state),
    ) {
        let _ = storage.set_item(SESSION_KEY, &json);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn clear_session() {
    if let Some(storage) = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
    {
        let _ = storage.remove_item(SESSION_KEY);
    }
}
