use crate::errors::{AppError, Result};
use serde::Deserialize;
use std::time::Duration;

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
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    result: Option<T>,
    msg: Option<String>,
    error_code: Option<String>,
}
impl<T> ApiResponse<T> {
    fn into_result(self) -> Result<T> {
        if !self.success {
            let code = self.error_code.as_deref().unwrap_or("unknown");
            let msg = self.msg.as_deref().unwrap_or("erreur inconnue");
            return Err(AppError::FreeboxApi(format!("{code}: {msg}")));
        }

        self.result
            .ok_or_else(|| AppError::FreeboxApi("champ result manquant".to_string()))
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
        let api_base=format!("{base_url}/{api_prefix}/v{version}");
        Ok(Self{http, base_url, api_base, session_token:None})
    }
}
