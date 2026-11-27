use crate::errors::{AppError, Result};
use serde::{Deserialize, de::DeserializeOwned};
use std::time::Duration;

const TMDB_API_BASE: &str = "https://api.themoviedb.org/3";
const TMDB_IMAGE_BASE: &str = "https://image.tmdb.org/t/p";
const TMDB_LANGUAGE: &str = "fr-FR";
#[derive(Debug)]
pub struct TmdbClient {
    http: reqwest::Client,
    token: String,
    base_url: String,
}
impl TmdbClient {
    pub fn new(token: impl Into<String>) -> Result<Self> {
        Self::with_base_url(token, TMDB_API_BASE)
    }
    fn with_base_url(token: impl Into<String>, base_url: &str) -> Result<Self> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .build()?;
        Ok(Self {
            http,
            token: token.into(),
            base_url: base_url.trim_end_matches('/').into(),
        })
    }
}
#[derive(Debug, Clone, Deserialize)]
struct SearchResponse<T> {
    results: Vec<T>,
}
#[derive(Debug, Clone, Deserialize)]
struct TmdbErrorsResponse {
    status_message: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct TmdbMovie {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub overview: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub release_date: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct TmdbTvShow {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub overview: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub first_air_date: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct TmdbEpisode {
    pub season_number: u64,
    pub episode_number: u64,
    #[serde(default)]
    pub overview: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TmdbSeason {
    pub season_number: u64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub episodes: Vec<TmdbEpisode>,
}
fn image_url(path: Option<&str>, size: &str) -> String {
    match path {
        None | Some("") => String::new(),
        Some(p) => format!("{TMDB_IMAGE_BASE}/{size}{p}"),
    }
}
pub fn poster_url(path: Option<&str>) -> String {
    image_url(path, "w500")
}
pub fn backdrop_url(path: Option<&str>) -> String {
    image_url(path, "w1280")
}

impl TmdbClient {
    async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let req = self
            .http
            .get(&url)
            .bearer_auth(&self.token)
            .query(query)
            .send()
            .await?;
        let status = req.status();
        let body = req.text().await?;
        if !status.is_success() {
            let msg = serde_json::from_str::<TmdbErrorsResponse>(&body)
                .ok()
                .and_then(|e| e.status_message)
                .unwrap_or(body);
            Err(AppError::Tmdb(format!("HTTP {status}: {msg}")))
        } else {
            Ok(serde_json::from_str(&body)?)
        }
    }
    pub async fn search_movie(
        &self,
        title: impl Into<String>,
        year: Option<u16>,
    ) -> Result<Vec<TmdbMovie>> {
        let mut query = vec![
            ("query", title.into()),
            ("language", TMDB_LANGUAGE.to_string()),
        ];
        if let Some(y) = year {
            query.push(("year", y.to_string()));
        }
        let res = self
            .get_json::<SearchResponse<TmdbMovie>>("/search/movie", &query)
            .await?;
        Ok(res.results)
    }
    pub async fn search_tv(
        &self,
        title: impl Into<String>,
        year: Option<u16>,
    ) -> Result<Vec<TmdbTvShow>> {
        let mut query = vec![
            ("query", title.into()),
            ("language", TMDB_LANGUAGE.to_string()),
        ];
        if let Some(y) = year {
            query.push(("first_air_date_year", y.to_string()));
        }
        let res = self
            .get_json::<SearchResponse<TmdbTvShow>>("/search/tv", &query)
            .await?;
        Ok(res.results)
    }
    pub async fn get_season(&self, tv_id: u64, season_number: u64) -> Result<TmdbSeason> {
        let path = format!("/tv/{tv_id}/season/{season_number}");
        let query = [("language", TMDB_LANGUAGE.to_string())];
        self.get_json(&path, &query).await
    }
}

#[cfg(test)]
#[path = "tests/tmdb.rs"]
mod tests;
