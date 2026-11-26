#[expect(dead_code)]
pub mod firestore;
mod firestore_client;

use crate::errors::{AppError, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, Instant};

const IDENTITY_TOOLKIT_BASE: &str = "https://identitytoolkit.googleapis.com";
const SECURE_TOKEN_BASE: &str = "https://securetoken.googleapis.com";
const FIRESTORE_BASE: &str = "https://firestore.googleapis.com";
const TOKEN_REFRESH_MARGIN_SECS: u64 = 60;
#[derive(Debug)]
pub struct FirebaseClient {
    http: reqwest::Client,
    project_id: String,
    api_key: String,
    auth_base_url: String,
    token_base_url: String,
    firestore_base_url: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires_at: Option<Instant>,
}
#[derive(Deserialize)]
struct FirebaseErrorEnvelope {
    error: Option<FirebaseErrorDetail>,
}

#[derive(Deserialize)]
struct FirebaseErrorDetail {
    message: Option<String>,
}
async fn parse_firebase_response<T: DeserializeOwned>(
    resp: reqwest::Response,
    make_err: fn(String) -> AppError,
) -> Result<T> {
    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        let msg = serde_json::from_str::<FirebaseErrorEnvelope>(&body)
            .ok()
            .and_then(|e| e.error)
            .and_then(|e| e.message)
            .unwrap_or(body);
        return Err(make_err(format!("HTTP {status}: {msg}")));
    }
    Ok(serde_json::from_str(&body)?)
}
fn compute_expiry(expires_in: &str) -> Result<Instant> {
    let secs = expires_in
        .parse::<u64>()
        .map_err(|e| AppError::FirebaseAuth(format!("expires_in invalide ({expires_in}): {e}")))?;
    Ok(Instant::now() + Duration::from_secs(secs.saturating_sub(TOKEN_REFRESH_MARGIN_SECS)))
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SignInRequest<'a> {
    email: &'a str,
    password: &'a str,
    return_secure_token: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignInResponse {
    id_token: String,
    refresh_token: String,
    expires_in: String,
}
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct RefreshTokenRequest<'a> {
    grant_type: &'a str,
    refresh_token: &'a str,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct RefreshTokenResponse {
    id_token: String,
    refresh_token: String,
    expires_in: String,
}

impl FirebaseClient {
    pub fn new(project_id: impl Into<String>, api_key: impl Into<String>) -> Result<Self> {
        Self::with_base_urls(
            project_id,
            api_key,
            IDENTITY_TOOLKIT_BASE,
            SECURE_TOKEN_BASE,
            FIRESTORE_BASE,
        )
    }
    fn with_base_urls(
        project_id: impl Into<String>,
        api_key: impl Into<String>,
        auth_base_url: &str,
        token_base_url: &str,
        firestore_base_url: &str,
    ) -> Result<Self> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .build()?;

        Ok(Self {
            http,
            project_id: project_id.into(),
            api_key: api_key.into(),
            auth_base_url: auth_base_url.trim_end_matches('/').into(),
            token_base_url: token_base_url.trim_end_matches('/').into(),
            firestore_base_url: firestore_base_url.trim_end_matches('/').into(),
            refresh_token: None,
            id_token: None,
            expires_at: None,
        })
    }
    #[must_use]
    pub fn with_refresh_token(mut self, token: impl Into<String>) -> Self {
        self.refresh_token = Some(token.into());
        self
    }
    pub async fn sign_in(&mut self, email: &str, password: &str) -> Result<String> {
        let res = self
            .http
            .post(format!(
                "{}/v1/accounts:signInWithPassword",
                self.auth_base_url
            ))
            .query(&[("key", self.api_key.as_str())])
            .json(&SignInRequest {
                email,
                password,
                return_secure_token: true,
            })
            .send()
            .await?;
        let data = parse_firebase_response::<SignInResponse>(res, AppError::FirebaseAuth).await?;
        let end = compute_expiry(&data.expires_in)?;
        self.refresh_token = Some(data.refresh_token.clone());
        self.id_token = Some(data.id_token);
        self.expires_at = Some(end);
        Ok(data.refresh_token)
    }
    pub async fn refresh_id_token(&mut self) -> Result<()> {
        let Some(refresh_token) = self.refresh_token.as_deref() else {
            return Err(AppError::FirebaseAuth(
                "refresh token manquant...".to_string(),
            ));
        };
        let res = self
            .http
            .post(format!("{}/v1/token", self.token_base_url))
            .query(&[("key", self.api_key.as_str())])
            .json(&RefreshTokenRequest {
                grant_type: "refresh_token",
                refresh_token,
            })
            .send()
            .await?;
        let data =
            parse_firebase_response::<RefreshTokenResponse>(res, AppError::FirebaseAuth).await?;
        let expires_at = compute_expiry(&data.expires_in)?;
        self.refresh_token = Some(data.refresh_token);
        self.expires_at = Some(expires_at);
        self.id_token = Some(data.id_token);
        Ok(())
    }
    pub(crate) async fn valid_id_token(&mut self) -> Result<&str> {
        let refresh_required =
            self.id_token.is_none() || self.expires_at.is_none_or(|exp| Instant::now() >= exp);
        if refresh_required {
            self.refresh_id_token().await?;
        }
        self.id_token.as_deref().ok_or_else(|| {
            AppError::FirebaseAuth("impossible de valider l'id du token".to_string())
        })
    }
}

#[cfg(test)]
#[path = "../tests/firebase.rs"]
mod tests;
