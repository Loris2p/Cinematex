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
fn default_secrets_has_all_fields_none() {
    let secrets = Secrets::default();
    assert!(secrets.app_token.is_none());
    assert!(secrets.firebase_refresh_token.is_none());
}

#[test]
fn secrets_debug_redacts_tokens() {
    let populated = Secrets {
        app_token: Some("super_secret_fbx".to_string()),
        firebase_refresh_token: Some("super_secret_fb".to_string()),
    };
    let dbg = format!("{populated:?}");
    assert!(!dbg.contains("super_secret_fbx"));
    assert!(!dbg.contains("super_secret_fb"));
    assert!(dbg.contains("[REDACTED]"));

    let empty = Secrets::default();
    let dbg_empty = format!("{empty:?}");
    assert!(!dbg_empty.contains("[REDACTED]"));
    assert!(dbg_empty.contains("None"));
}

#[test]
fn secrets_zeroize_clears_tokens() {
    let mut secrets = Secrets {
        app_token: Some("fbx_tok".to_string()),
        firebase_refresh_token: Some("fb_refresh".to_string()),
    };
    secrets.zeroize();
    assert!(secrets.app_token.is_none());
    assert!(secrets.firebase_refresh_token.is_none());
}

#[test]
fn secrets_json_roundtrip_preserves_all_fields() {
    let secrets = Secrets {
        app_token: Some("fbx_tok".to_string()),
        firebase_refresh_token: Some("fb_refresh".to_string()),
    };

    let json = serde_json::to_string_pretty(&secrets).unwrap();
    let decoded: Secrets = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded.app_token.as_deref(), Some("fbx_tok"));
    assert_eq!(
        decoded.firebase_refresh_token.as_deref(),
        Some("fb_refresh")
    );
}

#[test]
fn secrets_ignores_legacy_media_root_field_in_json() {
    let legacy = r#"{
        "app_token": "legacy_token",
        "firebase_refresh_token": null,
        "media_root": "/Freebox/Cinematex"
    }"#;
    let decoded: Secrets = serde_json::from_str(legacy).unwrap();
    assert_eq!(decoded.app_token.as_deref(), Some("legacy_token"));
    assert!(decoded.firebase_refresh_token.is_none());
}

#[test]
fn file_secret_store_returns_default_when_file_missing() {
    let dir = unique_temp_dir("missing");
    let store = FileSecretStore::new(dir.join("config.json"));
    let secrets = store.load().unwrap();
    assert!(secrets.app_token.is_none());
    assert!(secrets.firebase_refresh_token.is_none());
}

#[test]
fn file_secret_store_creates_parents_sets_0600_and_roundtrips() {
    let dir = unique_temp_dir("save_roundtrip");
    let path = dir.join("nested").join("config.json");
    let store = FileSecretStore::new(path.clone());

    let secrets = Secrets {
        app_token: Some("secret_app_token".to_string()),
        firebase_refresh_token: Some("secret_refresh".to_string()),
    };
    store.save(&secrets).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "config.json must have 0600 permissions");
    }

    let loaded = store.load().unwrap();
    assert_eq!(loaded.app_token.as_deref(), Some("secret_app_token"));
    assert_eq!(
        loaded.firebase_refresh_token.as_deref(),
        Some("secret_refresh")
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn file_secret_store_returns_json_error_on_corrupted_file() {
    let dir = unique_temp_dir("corrupted");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.json");
    fs::write(&path, "{ invalid json").unwrap();

    let store = FileSecretStore::new(path);
    let err = store.load().unwrap_err();
    assert!(matches!(err, AppError::Json(_)));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn embedded_config_toml_loads_all_expected_values() {
    let settings = Settings::load().expect("cli/config.toml must deserialize cleanly");

    assert_eq!(settings.http.connect_timeout_secs, 5);
    assert_eq!(settings.http.request_timeout_secs, 15);

    assert_eq!(settings.freebox.base_url, "http://mafreebox.freebox.fr");
    assert!(settings.freebox.media_root.is_none());

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
fn settings_from_toml_parses_custom_values_and_optional_media_root() {
    let custom = r#"
[http]
connect_timeout_secs = 2
request_timeout_secs = 10

[freebox]
base_url = "http://192.168.1.254"
media_root = "/Disque dur/Films"

[firebase]
auth_base_url = "http://127.0.0.1:9001"
token_base_url = "http://127.0.0.1:9002"
firestore_base_url = "http://127.0.0.1:9003"

[tmdb]
base_url = "http://127.0.0.1:9004"
image_base_url = "http://127.0.0.1:9005"
language = "en-US"

[indexer]
video_extensions = ["mkv", "webm"]
max_concurrency = 8

[ui]
language = "en"
refresh_interval_ms = 500
"#;
    let settings = Settings::from_toml(custom).unwrap();
    assert_eq!(settings.http.connect_timeout_secs, 2);
    assert_eq!(settings.http.request_timeout_secs, 10);
    assert_eq!(settings.freebox.base_url, "http://192.168.1.254");
    assert_eq!(
        settings.freebox.media_root.as_deref(),
        Some("/Disque dur/Films")
    );
    assert_eq!(settings.tmdb.language, "en-US");
    assert_eq!(settings.indexer.video_extensions, vec!["mkv", "webm"]);
    assert_eq!(settings.indexer.max_concurrency, 8);
    assert_eq!(settings.ui.language, "en");
    assert_eq!(settings.ui.refresh_interval_ms, 500);
}

#[test]
fn settings_from_toml_returns_toml_error_on_invalid_input() {
    let err = Settings::from_toml("[freebox\nbase_url = ").unwrap_err();
    assert!(matches!(err, AppError::TomlDe(_)));
}
