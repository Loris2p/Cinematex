use crate::errors::{AppError, Result};

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
