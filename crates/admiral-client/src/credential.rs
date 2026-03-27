//! Credential storage abstraction. This phase: plain file; later: encrypted file (§19.5).

use crate::config::ClientConfig;
use admiral_types::api::TokenResponse;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

/// Stored credential (plain JSON for this phase).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredCredential {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub user_email: Option<String>,
    #[serde(default)]
    pub user_name: Option<String>,
    #[serde(default)]
    pub user_role: Option<String>,
}

/// Abstraction for reading/writing credentials. Implementations: plain file now, encrypted file later.
pub trait CredentialStore: Send + Sync {
    fn get(&self) -> Result<Option<StoredCredential>, CredentialError>;
    fn set(&self, cred: &StoredCredential) -> Result<(), CredentialError>;
    fn clear(&self) -> Result<(), CredentialError>;
}

/// Plain-file implementation (this phase).
pub struct PlainFileCredentialStore {
    path: std::path::PathBuf,
}

impl PlainFileCredentialStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn from_config(config: &ClientConfig) -> Self {
        Self::new(&config.credential_path)
    }
}

impl CredentialStore for PlainFileCredentialStore {
    fn get(&self) -> Result<Option<StoredCredential>, CredentialError> {
        let data = match std::fs::read_to_string(&self.path) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let cred: StoredCredential = serde_json::from_str(&data)?;
        Ok(Some(cred))
    }

    fn set(&self, cred: &StoredCredential) -> Result<(), CredentialError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(cred)?;
        std::fs::write(&self.path, data)?;
        Ok(())
    }

    fn clear(&self) -> Result<(), CredentialError> {
        if self.path.exists() {
            std::fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}

impl StoredCredential {
    pub fn from_token_response(resp: &TokenResponse) -> Self {
        Self {
            access_token: resp.access_token.clone(),
            refresh_token: resp.refresh_token.clone(),
            user_id: Some(resp.user.id.clone()),
            user_email: Some(resp.user.email.clone()),
            user_name: Some(resp.user.name.clone()),
            user_role: Some(resp.user.role.clone()),
        }
    }
}
