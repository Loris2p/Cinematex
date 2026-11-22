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
    assert_eq!(decoded.firebase_refresh_token.as_deref(), Some("fb_refresh"));
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
