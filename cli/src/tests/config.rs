use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    env::temp_dir().join(format!(
        "cinematex_test_{label}_{}_{nanos}",
        std::process::id()
    ))
}

#[test]
fn default_config_has_all_fields_none() {
    let cfg = AppConfig::default();
    assert!(cfg.app_token.is_none());
    assert!(cfg.firebase_refresh_token.is_none());
    assert!(cfg.media_root.is_none());
}

#[test]
fn config_json_roundtrip_preserves_all_fields() {
    let cfg = AppConfig {
        app_token: Some("fbx_tok".to_string()),
        firebase_refresh_token: Some("fb_refresh".to_string()),
        media_root: Some("/Freebox/Cinematex".to_string()),
    };

    let json = serde_json::to_string_pretty(&cfg).unwrap();
    let decoded: AppConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded.app_token.as_deref(), Some("fbx_tok"));
    assert_eq!(
        decoded.firebase_refresh_token.as_deref(),
        Some("fb_refresh")
    );
    assert_eq!(decoded.media_root.as_deref(), Some("/Freebox/Cinematex"));
}

#[test]
fn config_deserializes_legacy_json_without_media_root() {
    let legacy = r#"{
        "app_token": "legacy_token",
        "firebase_refresh_token": null
    }"#;
    let decoded: AppConfig = serde_json::from_str(legacy).unwrap();
    assert_eq!(decoded.app_token.as_deref(), Some("legacy_token"));
    assert!(decoded.firebase_refresh_token.is_none());
    assert!(decoded.media_root.is_none());
}

#[test]
fn load_from_returns_default_when_file_missing() {
    let dir = unique_temp_dir("missing");
    let path = dir.join("config.json");
    let cfg = AppConfig::load_from(&path).unwrap();
    assert!(cfg.app_token.is_none());
    assert!(cfg.firebase_refresh_token.is_none());
    assert!(cfg.media_root.is_none());
}

#[test]
fn save_to_creates_parents_sets_0600_and_roundtrips() {
    let dir = unique_temp_dir("save_roundtrip");
    let path = dir.join("nested").join("config.json");

    let cfg = AppConfig {
        app_token: Some("secret_app_token".to_string()),
        firebase_refresh_token: Some("secret_refresh".to_string()),
        media_root: Some("/Freebox/Videos".to_string()),
    };
    cfg.save_to(&path).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "config.json must have 0600 permissions");
    }

    let loaded = AppConfig::load_from(&path).unwrap();
    assert_eq!(loaded.app_token.as_deref(), Some("secret_app_token"));
    assert_eq!(
        loaded.firebase_refresh_token.as_deref(),
        Some("secret_refresh")
    );
    assert_eq!(loaded.media_root.as_deref(), Some("/Freebox/Videos"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn load_from_returns_json_error_on_corrupted_file() {
    let dir = unique_temp_dir("corrupted");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.json");
    fs::write(&path, "{ invalid json").unwrap();

    let err = AppConfig::load_from(&path).unwrap_err();
    assert!(matches!(err, AppError::Json(_)));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn settings_default_has_expected_values() {
    let settings = Settings::default();

    assert!(settings.freebox.base_url.is_none());
    assert!(settings.freebox.media_root.is_none());
    assert_eq!(settings.freebox.connect_timeout_secs, 5);
    assert_eq!(settings.freebox.request_timeout_secs, 15);

    assert!(settings.firebase.project_id.is_none());
    assert_eq!(
        settings.firebase.auth_base_url,
        "https://identitytoolkit.googleapis.com"
    );
    assert_eq!(
        settings.firebase.token_base_url,
        "https://securetoken.googleapis.com"
    );
    assert_eq!(
        settings.firebase.firestore_base_url,
        "https://firestore.googleapis.com"
    );

    assert_eq!(settings.tmdb.base_url, "https://api.themoviedb.org/3");
    assert_eq!(settings.tmdb.image_base_url, "https://image.tmdb.org/t/p");
    assert_eq!(settings.tmdb.language, "fr-FR");

    assert_eq!(settings.indexer.video_extensions, vec!["mkv", "mp4", "avi"]);
    assert_eq!(settings.indexer.max_concurrency, 4);

    assert_eq!(settings.ui.language, "fr");
    assert_eq!(settings.ui.refresh_interval_ms, 1000);
}

#[test]
fn settings_load_from_returns_default_when_file_missing() {
    let dir = unique_temp_dir("settings_missing");
    let path = dir.join("config.toml");

    let settings = Settings::load_from(&path).unwrap();
    assert_eq!(settings.freebox.connect_timeout_secs, 5);
    assert_eq!(settings.tmdb.language, "fr-FR");
    assert_eq!(settings.indexer.max_concurrency, 4);
}

#[test]
fn settings_partial_toml_falls_back_to_defaults() {
    let dir = unique_temp_dir("settings_partial");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");

    let partial = r#"
[freebox]
media_root = "/Disque dur/Films"

[tmdb]
language = "en-US"

[indexer]
max_concurrency = 8
"#;
    fs::write(&path, partial).unwrap();

    let settings = Settings::load_from(&path).unwrap();
    assert_eq!(
        settings.freebox.media_root.as_deref(),
        Some("/Disque dur/Films")
    );
    assert!(settings.freebox.base_url.is_none());
    assert_eq!(settings.freebox.connect_timeout_secs, 5);
    assert_eq!(settings.freebox.request_timeout_secs, 15);

    assert_eq!(settings.tmdb.language, "en-US");
    assert_eq!(settings.tmdb.base_url, "https://api.themoviedb.org/3");

    assert_eq!(settings.indexer.max_concurrency, 8);
    assert_eq!(settings.indexer.video_extensions, vec!["mkv", "mp4", "avi"]);

    assert_eq!(settings.ui.language, "fr");
    assert_eq!(settings.ui.refresh_interval_ms, 1000);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn settings_save_to_and_load_from_roundtrip() {
    let dir = unique_temp_dir("settings_roundtrip");
    let path = dir.join("nested").join("config.toml");

    let mut settings = Settings::default();
    settings.freebox.base_url = Some("http://192.168.1.254".into());
    settings.freebox.media_root = Some("/Disque dur/Cinematex".into());
    settings.firebase.project_id = Some("my-project".into());
    settings.tmdb.language = "en-US".into();
    settings.indexer.video_extensions = vec!["mkv".into(), "webm".into()];
    settings.indexer.max_concurrency = 6;
    settings.ui.language = "en".into();
    settings.ui.refresh_interval_ms = 500;

    settings.save_to(&path).unwrap();

    let loaded = Settings::load_from(&path).unwrap();
    assert_eq!(
        loaded.freebox.base_url.as_deref(),
        Some("http://192.168.1.254")
    );
    assert_eq!(
        loaded.freebox.media_root.as_deref(),
        Some("/Disque dur/Cinematex")
    );
    assert_eq!(loaded.firebase.project_id.as_deref(), Some("my-project"));
    assert_eq!(loaded.tmdb.language, "en-US");
    assert_eq!(loaded.indexer.video_extensions, vec!["mkv", "webm"]);
    assert_eq!(loaded.indexer.max_concurrency, 6);
    assert_eq!(loaded.ui.language, "en");
    assert_eq!(loaded.ui.refresh_interval_ms, 500);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn settings_load_from_returns_toml_error_on_invalid_file() {
    let dir = unique_temp_dir("settings_invalid");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    fs::write(&path, "[freebox\nbase_url = ").unwrap();

    let err = Settings::load_from(&path).unwrap_err();
    assert!(matches!(err, AppError::TomlDe(_)));

    let _ = fs::remove_dir_all(&dir);
}
