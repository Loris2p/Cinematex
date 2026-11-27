use crate::errors::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{env, fs};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FreeboxSettings {
    pub base_url: Option<String>,
    pub media_root: Option<String>,
    pub connect_timeout_secs: u64,
    pub request_timeout_secs: u64,
}
impl Default for FreeboxSettings {
    fn default() -> Self {
        Self {
            base_url: None,
            media_root: None,
            connect_timeout_secs: 5,
            request_timeout_secs: 15,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FirebaseSettings {
    pub project_id: Option<String>,
    pub auth_base_url: String,
    pub token_base_url: String,
    pub firestore_base_url: String,
}
impl Default for FirebaseSettings {
    fn default() -> Self {
        Self {
            project_id: None,
            auth_base_url: "https://identitytoolkit.googleapis.com".into(),
            token_base_url: "https://securetoken.googleapis.com".into(),
            firestore_base_url: "https://firestore.googleapis.com".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TmdbSettings {
    pub base_url: String,
    pub image_base_url: String,
    pub language: String,
}
impl Default for TmdbSettings {
    fn default() -> Self {
        Self {
            base_url: "https://api.themoviedb.org/3".into(),
            image_base_url: "https://image.tmdb.org/t/p".into(),
            language: "fr-FR".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IndexerSettings {
    pub video_extensions: Vec<String>,
    pub max_concurrency: usize,
}
impl Default for IndexerSettings {
    fn default() -> Self {
        Self {
            video_extensions: vec!["mkv".into(), "mp4".into(), "avi".into()],
            max_concurrency: 4,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    pub language: String,
    pub refresh_interval_ms: u64,
}
impl Default for UiSettings {
    fn default() -> Self {
        Self {
            language: "fr".into(),
            refresh_interval_ms: 1000,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Settings {
    pub freebox: FreeboxSettings,
    pub firebase: FirebaseSettings,
    pub tmdb: TmdbSettings,
    pub indexer: IndexerSettings,
    pub ui: UiSettings,
}
impl Settings {
    pub fn load() -> Result<Self> {
        let path = Self::settings_path()?;
        Self::load_from(&path)
    }
    fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            Ok(Self::default())
        } else {
            let settings_str = fs::read_to_string(path)?;
            Ok(toml::from_str(&settings_str)?)
        }
    }
    pub fn save(&self) -> Result<()> {
        let path = Self::settings_path()?;
        self.save_to(&path)
    }
    fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let toml_str = toml::to_string_pretty(self)?;
        fs::write(path, toml_str)?;
        Ok(())
    }
    fn settings_path() -> Result<PathBuf> {
        Ok(config_dir()?.join("config.toml"))
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub app_token: Option<String>,
    pub firebase_refresh_token: Option<String>,
    #[serde(default)]
    pub media_root: Option<String>,
}

impl AppConfig {
    fn config_path() -> Result<PathBuf> {
        Ok(config_dir()?.join("config.json"))
    }
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        Self::load_from(&path)
    }
    fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            Ok(Self::default())
        } else {
            let conf_str = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&conf_str)?)
        }
    }
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        self.save_to(&path)
    }
    fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }
}
pub fn firebase_env() -> Result<(&'static str, &'static str)> {
    let Some(fb_project_id) = option_env!("FIREBASE_PROJECT_ID").filter(|s| !s.is_empty()) else {
        return Err(AppError::Config(
            "Clef FIREBASE_PROJECT_ID manquante dans le .env".to_string(),
        ));
    };
    let Some(fb_api_key) = option_env!("FIREBASE_WEB_API_KEY").filter(|s| !s.is_empty()) else {
        return Err(AppError::Config(
            "Clef FIREBASE_WEB_API_KEY manquante dans le .env".to_string(),
        ));
    };
    Ok((fb_project_id, fb_api_key))
}
pub fn freebox_env() -> Result<&'static str> {
    let Some(res) = option_env!("FREEBOX_URL").filter(|s| !s.is_empty()) else {
        return Err(AppError::Config(
            "Clef FREEBOX_URL manquante dans le .env".to_string(),
        ));
    };
    Ok(res)
}
pub fn tmdb_token() -> Result<&'static str> {
    let Some(res) = option_env!("TMDB_READING_TOKEN").filter(|s| !s.is_empty()) else {
        return Err(AppError::Config(
            "Clef TMDB_READING_TOKEN manquante dans le .env".to_string(),
        ));
    };
    Ok(res)
}

fn config_dir() -> Result<PathBuf> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map_err(|_| {
            AppError::Config(
                "Variable d'environnement HOME OU USERPROFILE introuvables".to_string(),
            )
        })?;
    Ok(PathBuf::from(home).join(".config/cinematex"))
}
#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
