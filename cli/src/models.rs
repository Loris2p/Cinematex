use crate::errors::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Movie,
    Series,
}
impl MediaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Series => "series",
            MediaType::Movie => "movie",
        }
    }
}
impl FromStr for MediaType {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "series" => Ok(Self::Series),
            "movie" => Ok(Self::Movie),
            other => Err(AppError::Firestore(format!("type media inconnu: {other}"))),
        }
    }
}
impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RequestStatus {
    Pending,
    Approved,
    Available,
    Rejected,
}
impl RequestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RequestStatus::Pending => "pending",
            RequestStatus::Approved => "approved",
            RequestStatus::Available => "available",
            RequestStatus::Rejected => "rejected",
        }
    }
}
impl FromStr for RequestStatus {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(RequestStatus::Pending),
            "approved" => Ok(RequestStatus::Approved),
            "available" => Ok(RequestStatus::Available),
            "rejected" => Ok(RequestStatus::Rejected),
            other => Err(AppError::Firestore(format!("status inconnu {other}"))),
        }
    }
}
impl std::fmt::Display for RequestStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub id: Option<String>,
    pub name: String,
    pub avatar_url: String,
    pub is_kid: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Media {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub media_type: MediaType,
    pub title: String,
    pub tmdb_id: u64,
    pub overview: String,
    pub poster_url: String,
    pub backdrop_url: String,
    pub allowed_profiles: Vec<String>,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    pub id: Option<String>,
    pub season: u64,
    pub episode: u64,
    pub title: String,
    pub file_path: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct MediaRequest {
    pub id: Option<String>,
    pub profile_id: String,
    pub tmdb_id: Option<u64>,
    #[serde(rename = "type")]
    pub media_type: MediaType,
    pub title: String,
    pub poster_url: Option<String>,
    pub status: RequestStatus,
    pub created_at: Option<String>,
}

#[cfg(test)]
#[path = "tests/models.rs"]
mod tests;
