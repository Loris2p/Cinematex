use crate::errors::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{env, fs};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub app_token: Option<String>,
    pub firebase_refresh_token: Option<String>,
    #[serde(default)]
    pub media_root: Option<String>,
}

impl AppConfig {
    fn config_path() -> Result<PathBuf> {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .map_err(|_| {
                AppError::Config(
                    "Variable d'environnement HOME OU USERPROFILE introuvables".to_string(),
                )
            })?;
        Ok(PathBuf::from(home).join(".config/cinematex/config.json"))
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

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
