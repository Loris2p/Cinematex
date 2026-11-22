use crate::errors::{AppError, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use std::time::Duration;
use tokio::time;

type HmacSha1 = Hmac<Sha1>;
const APP_ID: &str = "fr.cinematex.cli";
const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTH_HEADER: &str = "X-Fbx-App-Auth";
#[derive(Debug)]
pub struct FreeboxClient {
    http: reqwest::Client,
    base_url: String,
    api_base: String,
    session_token: Option<String>,
}
#[derive(Debug, Deserialize)]
struct ApiVersionResponse {
    api_version: String,
    api_base_url: String,
}
#[derive(Serialize)]
struct AuthorizeRequest<'a> {
    app_id: &'a str,
    app_name: &'a str,
    app_version: &'a str,
    device_name: &'a str,
}
#[derive(Debug, Deserialize)]
struct AuthorizeResponse {
    app_token: String,
    track_id: u64,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AuthorizeStatus {
    Unknown,
    Pending,
    Timeout,
    Granted,
    Denied,
}
#[derive(Debug, Deserialize)]
struct AuthorizeStatusResponse {
    status: AuthorizeStatus,
}
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    result: Option<T>,
    msg: Option<String>,
    error_code: Option<String>,
}
impl<T> ApiResponse<T> {
    fn into_result(self) -> Result<T> {
        self.into_optional_result()?
            .ok_or_else(|| AppError::FreeboxApi("champ result manquant".to_string()))
    }
    fn into_optional_result(self) -> Result<Option<T>> {
        if !self.success {
            let code = self.error_code.as_deref().unwrap_or("unknown");
            let msg = self.msg.as_deref().unwrap_or("erreur inconnue");
            return Err(AppError::FreeboxApi(format!("{code}: {msg}")));
        }
        Ok(self.result)
    }
}
#[derive(Debug, Deserialize)]
struct LoginResponse {
    challenge: String,
}
#[derive(Serialize)]
struct SessionRequest<'a> {
    app_id: &'a str,
    password: &'a str,
}
#[derive(Debug, Deserialize)]
struct SessionResponse {
    session_token: String,
}
fn compute_password(app_token: &str, challenge: &str) -> Result<String> {
    let mut mac = HmacSha1::new_from_slice(app_token.as_bytes())
        .map_err(|e| AppError::FreeboxAuth(format!("clef hmac invalide : {e}")))?;
    mac.update(challenge.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileType {
    File,
    Dir,
}
#[derive(Debug, Clone, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub file_type: FileType,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub hidden: bool,
}
impl FileInfo {
    pub fn decoded_path(&self) -> Result<String> {
        let bytes = STANDARD.decode(&self.path)?;
        let res = String::from_utf8(bytes)
            .map_err(|e| AppError::FreeboxApi(format!("chemin utf-8 invalide : {e}")))?;
        Ok(res)
    }
}
impl FreeboxClient {
    pub async fn connect(base_url: &str) -> Result<Self> {
        let base_url = base_url.trim_end_matches('/').to_string();
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .build()?;
        let url = format!("{base_url}/api_version");
        let version_info = http
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json::<ApiVersionResponse>()
            .await?;
        let Some(version) = version_info
            .api_version
            .split('.')
            .next()
            .filter(|s| !s.is_empty())
        else {
            return Err(AppError::FreeboxApi(format!(
                "version d'api invalide : {}",
                version_info.api_version
            )));
        };
        let api_prefix = version_info.api_base_url.trim_matches('/');
        let api_base = format!("{base_url}/{api_prefix}/v{version}");
        Ok(Self {
            http,
            base_url,
            api_base,
            session_token: None,
        })
    }
    pub async fn authorize(&self, device_name: &str) -> Result<String> {
        let url = format!("{}/login/authorize/", self.api_base);
        let authorize_request = AuthorizeRequest {
            app_id: APP_ID,
            app_name: APP_NAME,
            app_version: APP_VERSION,
            device_name,
        };
        let auth = self
            .http
            .post(&url)
            .json(&authorize_request)
            .send()
            .await?
            .error_for_status()?
            .json::<ApiResponse<AuthorizeResponse>>()
            .await?
            .into_result()?;
        println!("Merci de confirmer la connexion physiquement sur la freebox...");
        let status_url = format!("{}/login/authorize/{}", self.api_base, auth.track_id);
        loop {
            let auth_response = self
                .http
                .get(&status_url)
                .send()
                .await?
                .error_for_status()?
                .json::<ApiResponse<AuthorizeStatusResponse>>()
                .await?
                .into_result()?;

            match auth_response.status {
                AuthorizeStatus::Pending => time::sleep(Duration::from_secs(2)).await,
                AuthorizeStatus::Granted => return Ok(auth.app_token),
                AuthorizeStatus::Unknown => {
                    return Err(AppError::FreeboxAuth(
                        "demande authorisation inconnue ou expirée".to_string(),
                    ));
                }
                AuthorizeStatus::Timeout => {
                    return Err(AppError::FreeboxAuth(
                        "délai authorisation expiré".to_string(),
                    ));
                }
                AuthorizeStatus::Denied => {
                    return Err(AppError::FreeboxAuth(
                        "autorisation refusée sur la freebox".to_string(),
                    ));
                }
            }
        }
    }
    pub async fn open_session(&mut self, app_token: &str) -> Result<()> {
        let login = self
            .http
            .get(format!("{}/login/", self.api_base))
            .send()
            .await?
            .error_for_status()?
            .json::<ApiResponse<LoginResponse>>()
            .await?
            .into_result()?;
        let password = compute_password(app_token, &login.challenge)?;
        let session_request = SessionRequest {
            app_id: APP_ID,
            password: &password,
        };

        let session = self
            .http
            .post(format!("{}/login/session/", self.api_base))
            .json(&session_request)
            .send()
            .await?
            .error_for_status()?
            .json::<ApiResponse<SessionResponse>>()
            .await?
            .into_result()?;
        self.session_token = Some(session.session_token);
        Ok(())
    }
    fn require_session_token(&self) -> Result<&str> {
        self.session_token
            .as_deref()
            .ok_or_else(|| AppError::FreeboxAuth("session freebox non ouverte".to_string()))
    }
    pub async fn list_dir(&self, path: &str) -> Result<Vec<FileInfo>> {
        let token = self.require_session_token()?;
        let encoded = STANDARD.encode(path);
        let res = self
            .http
            .get(format!("{}/fs/ls/{encoded}", self.api_base))
            .header(AUTH_HEADER, token)
            .send()
            .await?
            .error_for_status()?
            .json::<ApiResponse<Vec<FileInfo>>>()
            .await?
            .into_optional_result()?
            .unwrap_or_default();
        Ok(res)
    }
}
