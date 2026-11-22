use super::*;
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

struct MockResponse {
    status: &'static str,
    body: String,
}

impl MockResponse {
    fn ok_json(value: serde_json::Value) -> Self {
        Self {
            status: "200 OK",
            body: value.to_string(),
        }
    }

    fn status(status: &'static str, body: &str) -> Self {
        Self {
            status,
            body: body.to_string(),
        }
    }
}

/// Starts a minimal single-use local HTTP/1.1 mock server serving a scripted sequence of responses.
/// Also captures each raw HTTP request received for assertions.
fn spawn_mock_server(responses: Vec<MockResponse>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let mut requests = Vec::with_capacity(responses.len());
        for resp in responses {
            let (mut stream, _) = listener.accept().expect("accept connection");
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut buf = [0_u8; 8192];
            let n = stream.read(&mut buf).unwrap_or(0);
            requests.push(String::from_utf8_lossy(&buf[..n]).into_owned());

            let raw_resp = format!(
                "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp.status,
                resp.body.len(),
                resp.body
            );
            stream
                .write_all(raw_resp.as_bytes())
                .expect("write response");
            stream.flush().ok();
        }
        requests
    });
    (format!("http://{addr}"), handle)
}

#[test]
fn compute_password_matches_rfc2202_hmac_sha1_vector() {
    let digest =
        compute_password("Jefe", "what do ya want for nothing?").expect("HMAC-SHA1 should succeed");
    assert_eq!(digest, "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79");
}

#[test]
fn compute_password_handles_empty_inputs_and_outputs_lowercase_hex() {
    let digest = compute_password("", "").expect("empty HMAC-SHA1 should succeed");
    assert_eq!(digest.len(), 40);
    assert!(
        digest
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
    assert_eq!(digest, "fbdb1d1b18aa6c08324b7d64b71fb76370690e1d");
}

#[test]
fn api_response_into_result_deserializes_valid_payload() {
    let raw = json!({
        "success": true,
        "result": { "challenge": "abc123" }
    });
    let resp: ApiResponse = serde_json::from_value(raw).unwrap();
    let login = resp.into_result::<LoginResponse>().unwrap();
    assert_eq!(login.challenge, "abc123");
}

#[test]
fn api_response_into_result_errors_when_result_missing() {
    let raw = json!({ "success": true });
    let resp: ApiResponse = serde_json::from_value(raw).unwrap();
    let err = resp.into_result::<LoginResponse>().unwrap_err();
    assert!(matches!(err, AppError::FreeboxApi(msg) if msg.contains("result manquant")));
}

#[test]
fn api_response_into_optional_result_returns_none_when_result_missing() {
    let raw = json!({ "success": true });
    let resp: ApiResponse = serde_json::from_value(raw).unwrap();
    let data = resp.into_optional_result::<ListDirResponse>().unwrap();
    assert!(data.is_none());
}

#[test]
fn api_response_reports_freebox_error_even_when_result_is_object() {
    let raw = json!({
        "success": false,
        "error_code": "insufficient_rights",
        "msg": "Vous n'avez pas les droits",
        "result": { "missing_right": "explorer" }
    });
    let resp: ApiResponse = serde_json::from_value(raw).unwrap();
    let err = resp.into_optional_result::<ListDirResponse>().unwrap_err();
    assert!(
        matches!(err, AppError::FreeboxApi(msg) if msg.contains("insufficient_rights") && msg.contains("droits"))
    );
}

#[test]
fn api_response_uses_fallback_error_code_and_message_when_absent() {
    let raw = json!({ "success": false });
    let resp: ApiResponse = serde_json::from_value(raw).unwrap();
    let err = resp.into_result::<LoginResponse>().unwrap_err();
    assert!(matches!(err, AppError::FreeboxApi(msg) if msg == "unknown: erreur inconnue"));
}

#[test]
fn api_response_returns_json_error_on_mismatched_success_payload() {
    let raw = json!({
        "success": true,
        "result": { "wrong_field": 42 }
    });
    let resp: ApiResponse = serde_json::from_value(raw).unwrap();
    let err = resp.into_result::<LoginResponse>().unwrap_err();
    assert!(matches!(err, AppError::Json(_)));
}

#[test]
fn authorize_status_deserializes_all_variants() {
    let cases = [
        (
            "\"unknown\"",
            matches!(
                serde_json::from_str::<AuthorizeStatus>("\"unknown\"").unwrap(),
                AuthorizeStatus::Unknown
            ),
        ),
        (
            "\"pending\"",
            matches!(
                serde_json::from_str::<AuthorizeStatus>("\"pending\"").unwrap(),
                AuthorizeStatus::Pending
            ),
        ),
        (
            "\"timeout\"",
            matches!(
                serde_json::from_str::<AuthorizeStatus>("\"timeout\"").unwrap(),
                AuthorizeStatus::Timeout
            ),
        ),
        (
            "\"granted\"",
            matches!(
                serde_json::from_str::<AuthorizeStatus>("\"granted\"").unwrap(),
                AuthorizeStatus::Granted
            ),
        ),
        (
            "\"denied\"",
            matches!(
                serde_json::from_str::<AuthorizeStatus>("\"denied\"").unwrap(),
                AuthorizeStatus::Denied
            ),
        ),
    ];
    for (label, matched) in cases {
        assert!(matched, "failed to match {label}");
    }
    assert!(serde_json::from_str::<AuthorizeStatus>("\"invalid\"").is_err());
}

#[test]
fn list_dir_response_deserializes_v16_entries_and_defaults() {
    let raw = json!({
        "entries": [
            {
                "name": "Freebox",
                "path": "L0ZyZWVib3g=",
                "type": "dir",
                "size": 4096,
                "hidden": false
            },
            {
                "name": "Inception.mkv",
                "path": "L0ZyZWVib3gvSW5jZXB0aW9uLm1rdg==",
                "type": "file"
            }
        ]
    });
    let parsed: ListDirResponse = serde_json::from_value(raw).unwrap();
    assert_eq!(parsed.entries.len(), 2);

    assert_eq!(parsed.entries[0].name, "Freebox");
    assert_eq!(parsed.entries[0].file_type, FileType::Dir);
    assert_eq!(parsed.entries[0].size, 4096);
    assert!(!parsed.entries[0].hidden);
    assert_eq!(parsed.entries[0].decoded_path().unwrap(), "/Freebox");

    assert_eq!(parsed.entries[1].name, "Inception.mkv");
    assert_eq!(parsed.entries[1].file_type, FileType::File);
    assert_eq!(parsed.entries[1].size, 0);
    assert!(!parsed.entries[1].hidden);
    assert_eq!(
        parsed.entries[1].decoded_path().unwrap(),
        "/Freebox/Inception.mkv"
    );
}

#[test]
fn list_dir_response_defaults_to_empty_vec_when_entries_missing() {
    let parsed: ListDirResponse = serde_json::from_value(json!({})).unwrap();
    assert!(parsed.entries.is_empty());
}

#[test]
fn file_info_decoded_path_handles_utf8_accents() {
    let original = "/Freebox/Séries/Épisode 01.mkv";
    let info = FileInfo {
        name: "Épisode 01.mkv".to_string(),
        path: STANDARD.encode(original),
        file_type: FileType::File,
        size: 1234,
        hidden: false,
    };
    assert_eq!(info.decoded_path().unwrap(), original);
}

#[test]
fn file_info_decoded_path_rejects_invalid_base64() {
    let info = FileInfo {
        name: "bad".to_string(),
        path: "!!!not_base64!!!".to_string(),
        file_type: FileType::File,
        size: 0,
        hidden: false,
    };
    let err = info.decoded_path().unwrap_err();
    assert!(matches!(err, AppError::Base64(_)));
}

#[test]
fn file_info_decoded_path_rejects_invalid_utf8_bytes() {
    let info = FileInfo {
        name: "bad_utf8".to_string(),
        path: STANDARD.encode([0xff_u8, 0xfe_u8]),
        file_type: FileType::File,
        size: 0,
        hidden: false,
    };
    let err = info.decoded_path().unwrap_err();
    assert!(matches!(err, AppError::FreeboxApi(msg) if msg.contains("utf-8")));
}

#[test]
fn require_session_token_checks_presence() {
    let mut client = FreeboxClient {
        http: reqwest::Client::new(),
        api_base: "http://mafreebox.freebox.fr/api/v16".to_string(),
        session_token: None,
    };
    assert!(matches!(
        client.require_session_token(),
        Err(AppError::FreeboxAuth(_))
    ));

    client.session_token = Some("tok_secret".to_string());
    assert_eq!(client.require_session_token().unwrap(), "tok_secret");
}

#[tokio::test]
async fn connect_parses_major_version_and_trims_trailing_slash() {
    let (base, handle) = spawn_mock_server(vec![MockResponse::ok_json(json!({
        "api_version": "16.1",
        "api_base_url": "/api/"
    }))]);

    let client = FreeboxClient::connect(&format!("{base}/")).await.unwrap();
    assert_eq!(client.api_base, format!("{base}/api/v16"));
    assert!(client.session_token.is_none());

    let reqs = handle.join().unwrap();
    assert!(reqs[0].starts_with("GET /api_version HTTP/"));
}

#[tokio::test]
async fn connect_rejects_empty_api_version() {
    let (base, handle) = spawn_mock_server(vec![MockResponse::ok_json(json!({
        "api_version": ".1",
        "api_base_url": "/api/"
    }))]);

    let err = FreeboxClient::connect(&base).await.unwrap_err();
    assert!(matches!(err, AppError::FreeboxApi(msg) if msg.contains("version d'api invalide")));
    handle.join().unwrap();
}

#[tokio::test]
async fn connect_propagates_http_status_error() {
    let (base, handle) = spawn_mock_server(vec![MockResponse::status(
        "500 Internal Server Error",
        "boom",
    )]);

    let err = FreeboxClient::connect(&base).await.unwrap_err();
    assert!(matches!(err, AppError::Http(_)));
    handle.join().unwrap();
}

#[tokio::test]
async fn authorize_returns_app_token_when_granted() {
    let (base, handle) = spawn_mock_server(vec![
        MockResponse::ok_json(json!({
            "api_version": "16.1",
            "api_base_url": "/api/"
        })),
        MockResponse::ok_json(json!({
            "success": true,
            "result": { "app_token": "tok_granted_42", "track_id": 7 }
        })),
        MockResponse::ok_json(json!({
            "success": true,
            "result": { "status": "granted" }
        })),
    ]);

    let client = FreeboxClient::connect(&base).await.unwrap();
    let token = client.authorize("TestBox").await.unwrap();
    assert_eq!(token, "tok_granted_42");

    let reqs = handle.join().unwrap();
    assert!(reqs[1].starts_with("POST /api/v16/login/authorize/ HTTP/"));
    assert!(reqs[1].contains("TestBox"));
    assert!(reqs[2].starts_with("GET /api/v16/login/authorize/7 HTTP/"));
}

#[tokio::test]
async fn authorize_returns_error_on_denied_timeout_and_unknown() {
    for status in ["denied", "timeout", "unknown"] {
        let (base, handle) = spawn_mock_server(vec![
            MockResponse::ok_json(json!({
                "api_version": "16.1",
                "api_base_url": "/api/"
            })),
            MockResponse::ok_json(json!({
                "success": true,
                "result": { "app_token": "tok_unused", "track_id": 9 }
            })),
            MockResponse::ok_json(json!({
                "success": true,
                "result": { "status": status }
            })),
        ]);

        let client = FreeboxClient::connect(&base).await.unwrap();
        let err = client.authorize("TestBox").await.unwrap_err();
        assert!(
            matches!(err, AppError::FreeboxAuth(_)),
            "expected FreeboxAuth for status {status}, got {err:?}"
        );
        handle.join().unwrap();
    }
}

#[tokio::test]
async fn open_session_sends_hmac_password_and_stores_token() {
    let app_token = "Jefe";
    let challenge = "what do ya want for nothing?";
    let expected_password = "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79";

    let (base, handle) = spawn_mock_server(vec![
        MockResponse::ok_json(json!({
            "api_version": "16.1",
            "api_base_url": "/api/"
        })),
        MockResponse::ok_json(json!({
            "success": true,
            "result": { "challenge": challenge }
        })),
        MockResponse::ok_json(json!({
            "success": true,
            "result": { "session_token": "sess_abc" }
        })),
    ]);

    let mut client = FreeboxClient::connect(&base).await.unwrap();
    client.open_session(app_token).await.unwrap();
    assert_eq!(client.require_session_token().unwrap(), "sess_abc");

    let reqs = handle.join().unwrap();
    assert!(reqs[1].starts_with("GET /api/v16/login/ HTTP/"));
    assert!(reqs[2].starts_with("POST /api/v16/login/session/ HTTP/"));
    assert!(reqs[2].contains(APP_ID));
    assert!(reqs[2].contains(expected_password));
}

#[tokio::test]
async fn list_dir_requires_open_session() {
    let client = FreeboxClient {
        http: reqwest::Client::new(),
        api_base: "http://127.0.0.1:1/api/v16".to_string(),
        session_token: None,
    };
    let err = client.list_dir("/").await.unwrap_err();
    assert!(matches!(err, AppError::FreeboxAuth(_)));
}

#[tokio::test]
async fn list_dir_sends_auth_header_and_filters_hidden_and_dot_entries() {
    let (base, handle) = spawn_mock_server(vec![MockResponse::ok_json(json!({
        "success": true,
        "result": {
            "entries": [
                { "name": ".", "path": "Lw==", "type": "dir", "hidden": true },
                { "name": "..", "path": "Lw==", "type": "dir", "hidden": true },
                { "name": ".Trash", "path": "Ly5UcmFzaA==", "type": "dir", "hidden": true },
                { "name": "Freebox", "path": "L0ZyZWVib3g=", "type": "dir", "size": 4096, "hidden": false },
                { "name": "Film.mkv", "path": "L0ZpbG0ubWt2", "type": "file", "size": 1024, "hidden": false }
            ]
        }
    }))]);

    let client = FreeboxClient {
        http: reqwest::Client::new(),
        api_base: format!("{base}/api/v16"),
        session_token: Some("sess_xyz".to_string()),
    };

    let entries = client.list_dir("/Freebox").await.unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "Freebox");
    assert_eq!(entries[0].file_type, FileType::Dir);
    assert_eq!(entries[1].name, "Film.mkv");
    assert_eq!(entries[1].file_type, FileType::File);

    let reqs = handle.join().unwrap();
    assert!(reqs[0].starts_with("GET /api/v16/fs/ls/L0ZyZWVib3g= HTTP/"));
    assert!(
        reqs[0]
            .to_ascii_lowercase()
            .contains("x-fbx-app-auth: sess_xyz")
    );
}

#[tokio::test]
async fn list_dir_handles_empty_result_and_api_errors() {
    let (base, handle) = spawn_mock_server(vec![
        MockResponse::ok_json(json!({ "success": true })),
        MockResponse::ok_json(json!({
            "success": false,
            "error_code": "insufficient_rights",
            "msg": "droits manquants",
            "result": { "missing_right": "explorer" }
        })),
    ]);

    let client = FreeboxClient {
        http: reqwest::Client::new(),
        api_base: format!("{base}/api/v16"),
        session_token: Some("sess_xyz".to_string()),
    };

    let empty = client.list_dir("/").await.unwrap();
    assert!(empty.is_empty());

    let err = client.list_dir("/").await.unwrap_err();
    assert!(matches!(err, AppError::FreeboxApi(msg) if msg.contains("insufficient_rights")));
    handle.join().unwrap();
}
