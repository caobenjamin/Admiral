//! JWT sign/verify (RS256) using PEM keys.

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,       // user_id
    pub exp: i64,          // expiry
    pub iat: i64,          // issued at
    pub token_type: String, // "access" | "refresh"
}

pub struct JwtKeys {
    encoding: EncodingKey,
    decoding: DecodingKey,
    validation: Validation,
}

impl JwtKeys {
    pub fn from_rsa_pem_files(private_path: &Path, public_path: &Path) -> Result<Self, anyhow::Error> {
        let private_pem = std::fs::read(private_path)
            .map_err(|e| anyhow::anyhow!("read private key {}: {}", private_path.display(), e))?;
        let public_pem = std::fs::read(public_path)
            .map_err(|e| anyhow::anyhow!("read public key {}: {}", public_path.display(), e))?;
        let encoding = EncodingKey::from_rsa_pem(&private_pem)
            .map_err(|e| anyhow::anyhow!("parse private key: {}", e))?;
        let decoding = DecodingKey::from_rsa_pem(&public_pem)
            .map_err(|e| anyhow::anyhow!("parse public key: {}", e))?;
        let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.validate_exp = true;
        validation.set_required_spec_claims(&["sub", "exp", "iat"]);
        Ok(Self {
            encoding,
            decoding,
            validation,
        })
    }

    pub fn sign(&self, claims: &Claims) -> Result<String, jsonwebtoken::errors::Error> {
        encode(&Header::new(jsonwebtoken::Algorithm::RS256), claims, &self.encoding)
    }

    pub fn verify(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let data = decode::<Claims>(token, &self.decoding, &self.validation)?;
        Ok(data.claims)
    }
}

pub fn build_claims(user_id: Uuid, token_type: &str, ttl_seconds: i64) -> Claims {
    let now = chrono::Utc::now().timestamp();
    Claims {
        sub: user_id.to_string(),
        exp: now + ttl_seconds,
        iat: now,
        token_type: token_type.to_string(),
    }
}
