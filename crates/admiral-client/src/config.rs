//! Client configuration: API base URL, credential file path.
//! Load order: default → config file (if present) → environment variables.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const DEFAULT_API_BASE_URL: &str = "http://127.0.0.1:8080";
const CONFIG_DIR_NAME: &str = "admiral";
const CONFIG_FILE_NAME: &str = "client.json";

/// Persisted fields in the config file (optional fields for backward compat).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientConfigFile {
    /// Server API base URL (e.g. https://admiral.example.com).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_base_url: Option<String>,
    /// Path to credential file (tokens stored here).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_path: Option<String>,
}

/// Client config loaded from file + env with defaults.
#[derive(Clone, Debug)]
pub struct ClientConfig {
    /// Server API base URL.
    pub api_base_url: String,
    /// Path to credential file.
    pub credential_path: PathBuf,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            api_base_url: DEFAULT_API_BASE_URL.to_string(),
            credential_path: PathBuf::from("credentials.json"),
        }
    }
}

impl ClientConfig {
    /// Path to the persistent config file (e.g. ~/.config/admiral/client.json).
    pub fn config_file_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join(CONFIG_DIR_NAME).join(CONFIG_FILE_NAME))
    }

    /// Load config: default → config file (if exists) → env overrides.
    pub fn from_env() -> Self {
        let mut c = Self::default();

        if let Some(path) = Self::config_file_path() {
            if path.is_file() {
                if let Ok(data) = std::fs::read_to_string(&path) {
                    if let Ok(file) = serde_json::from_str::<ClientConfigFile>(&data) {
                        if let Some(u) = file.api_base_url {
                            c.api_base_url = u.trim_end_matches('/').to_string();
                        }
                        if let Some(p) = file.credential_path {
                            c.credential_path = PathBuf::from(p);
                        }
                    }
                }
            }
        }

        if let Ok(u) = std::env::var("ADMIRAL_API_BASE_URL") {
            c.api_base_url = u.trim_end_matches('/').to_string();
        }
        if let Ok(p) = std::env::var("ADMIRAL_CREDENTIAL_FILE") {
            c.credential_path = PathBuf::from(p);
        }

        c
    }

    /// Save current api_base_url (and optional credential_path) to the config file.
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = match Self::config_file_path() {
            Some(p) => p,
            None => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "no config dir")),
        };
        let dir = path.parent().unwrap();
        std::fs::create_dir_all(dir)?;
        let file = ClientConfigFile {
            api_base_url: Some(self.api_base_url.clone()),
            credential_path: Some(self.credential_path.to_string_lossy().to_string()),
        };
        let json = serde_json::to_string_pretty(&file).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// Update api_base_url and persist to file.
    pub fn set_api_base_url(&mut self, url: String) {
        self.api_base_url = url.trim_end_matches('/').to_string();
    }
}
