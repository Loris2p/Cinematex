use crate::errors::Result;
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
}
