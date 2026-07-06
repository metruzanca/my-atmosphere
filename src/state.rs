#[derive(Debug, Clone, PartialEq, Default)]
pub struct SessionState {
    pub is_authenticated: bool,
    pub did: String,
    pub handle: String,
    pub pds_endpoint: String,
    pub access_token: String,
}
