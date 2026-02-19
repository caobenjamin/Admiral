//! Client configuration: API base URL, credential file path.

use std::path::PathBuf;

/// Client config loaded from env with defaults.
#[derive(Clone, Debug)]
pub struct ClientConfig {
    /// Server API base URL (e.g. http://127.0.0.1:8080).
    pub api_base_url: String,
    /// Path to credential file (tokens stored here by CredentialStore).
    pub credential_path: PathBuf,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            api_base_url: std::env::var("ADMIRAL_API_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string()),
            credential_path: PathBuf::from("credentials.json"),
        }
    }
}

impl ClientConfig {
    pub fn from_env() -> Self {
        let mut c = Self::default();
        if let Ok(u) = std::env::var("ADMIRAL_API_BASE_URL") {
            c.api_base_url = u;
        }
        if let Ok(p) = std::env::var("ADMIRAL_CREDENTIAL_FILE") {
            c.credential_path = PathBuf::from(p);
        }
        c
    }
}
