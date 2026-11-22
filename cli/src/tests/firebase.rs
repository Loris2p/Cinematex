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
fn compute_expiry_parses_valid_seconds_and_subtracts_margin() {
    let before = Instant::now();
    let expiry = compute_expiry("3600").expect("valid expires_in");
    let delta = expiry.saturating_duration_since(before);
    assert!(delta >= Duration::from_secs(3535) && delta <= Duration::from_secs(3545));
}

#[test]
fn compute_expiry_saturates_when_shorter_than_margin() {
    let before = Instant::now();
    let expiry = compute_expiry("30").expect("short expires_in should not underflow");
    assert!(expiry.saturating_duration_since(before) < Duration::from_secs(2));
}

#[test]
fn compute_expiry_rejects_non_numeric_input() {
    let err = compute_expiry("not-a-number").unwrap_err();
    assert!(matches!(err, AppError::FirebaseAuth(msg) if msg.contains("expires_in invalide")));
}

#[test]
fn with_base_urls_trims_trailing_slashes() {
    let client = FirebaseClient::with_base_urls(
        "cinematex-test",
        "api-key",
        "http://127.0.0.1:8001///",
        "http://127.0.0.1:8002/",
        "http://127.0.0.1:8003/",
    )
    .unwrap();
    assert_eq!(client.auth_base_url, "http://127.0.0.1:8001");
    assert_eq!(client.token_base_url, "http://127.0.0.1:8002");
    assert_eq!(client.firestore_base_url, "http://127.0.0.1:8003");
}

#[tokio::test]
async fn sign_in_sends_camel_case_payload_and_stores_tokens() {
    let (base, handle) = spawn_mock_server(vec![MockResponse::ok_json(json!({
        "idToken": "id-tok-1",
        "refreshToken": "ref-tok-1",
        "expiresIn": "3600"
    }))]);

    let mut client =
        FirebaseClient::with_base_urls("proj", "web-api-key", &base, &base, &base).unwrap();
    let refresh = client
        .sign_in("admin@cinematex.fr", "s3cr3t")
        .await
        .expect("sign_in should succeed");

    assert_eq!(refresh, "ref-tok-1");
    assert_eq!(client.id_token.as_deref(), Some("id-tok-1"));
    assert_eq!(client.refresh_token.as_deref(), Some("ref-tok-1"));
    assert!(client.expires_at.is_some());

    let reqs = handle.join().unwrap();
    assert_eq!(reqs.len(), 1);
    assert!(reqs[0].starts_with("POST /v1/accounts:signInWithPassword?key=web-api-key HTTP/1.1"));
    assert!(reqs[0].contains("\"email\":\"admin@cinematex.fr\""));
    assert!(reqs[0].contains("\"password\":\"s3cr3t\""));
    assert!(reqs[0].contains("\"returnSecureToken\":true"));
}

#[tokio::test]
async fn sign_in_extracts_firebase_error_message_on_http_error() {
    let err_body = json!({
        "error": {
            "code": 400,
            "message": "INVALID_LOGIN_CREDENTIALS"
        }
    })
    .to_string();
    let (base, handle) =
        spawn_mock_server(vec![MockResponse::status("400 Bad Request", &err_body)]);

    let mut client =
        FirebaseClient::with_base_urls("proj", "web-api-key", &base, &base, &base).unwrap();
    let err = client
        .sign_in("admin@cinematex.fr", "wrong")
        .await
        .unwrap_err();

    assert!(
        matches!(err, AppError::FirebaseAuth(msg) if msg.contains("400") && msg.contains("INVALID_LOGIN_CREDENTIALS"))
    );
    assert_eq!(handle.join().unwrap().len(), 1);
}

#[tokio::test]
async fn sign_in_falls_back_to_raw_body_on_non_json_error() {
    let (base, handle) = spawn_mock_server(vec![MockResponse::status(
        "502 Bad Gateway",
        "upstream down",
    )]);

    let mut client =
        FirebaseClient::with_base_urls("proj", "web-api-key", &base, &base, &base).unwrap();
    let err = client
        .sign_in("admin@cinematex.fr", "secret")
        .await
        .unwrap_err();

    assert!(
        matches!(err, AppError::FirebaseAuth(msg) if msg.contains("502") && msg.contains("upstream down"))
    );
    assert_eq!(handle.join().unwrap().len(), 1);
}

#[tokio::test]
async fn refresh_id_token_requires_refresh_token() {
    let mut client = FirebaseClient::with_base_urls(
        "proj",
        "web-api-key",
        "http://127.0.0.1:1",
        "http://127.0.0.1:1",
        "http://127.0.0.1:1",
    )
    .unwrap();

    let err = client.refresh_id_token().await.unwrap_err();
    assert!(matches!(err, AppError::FirebaseAuth(msg) if msg.contains("refresh token manquant")));
}

#[tokio::test]
async fn refresh_id_token_sends_snake_case_payload_and_updates_tokens() {
    let (base, handle) = spawn_mock_server(vec![MockResponse::ok_json(json!({
        "id_token": "fresh-id-tok",
        "refresh_token": "rotated-ref-tok",
        "expires_in": "3600"
    }))]);

    let mut client = FirebaseClient::with_base_urls("proj", "web-api-key", &base, &base, &base)
        .unwrap()
        .with_refresh_token("old-ref-tok");
    client
        .refresh_id_token()
        .await
        .expect("refresh should succeed");

    assert_eq!(client.id_token.as_deref(), Some("fresh-id-tok"));
    assert_eq!(client.refresh_token.as_deref(), Some("rotated-ref-tok"));
    assert!(client.expires_at.is_some());

    let reqs = handle.join().unwrap();
    assert_eq!(reqs.len(), 1);
    assert!(reqs[0].starts_with("POST /v1/token?key=web-api-key HTTP/1.1"));
    assert!(reqs[0].contains("\"grant_type\":\"refresh_token\""));
    assert!(reqs[0].contains("\"refresh_token\":\"old-ref-tok\""));
}

#[tokio::test]
async fn valid_id_token_returns_cached_token_without_network_call() {
    let (base, handle) = spawn_mock_server(vec![MockResponse::ok_json(json!({
        "idToken": "cached-id-tok",
        "refreshToken": "ref-tok",
        "expiresIn": "3600"
    }))]);

    let mut client =
        FirebaseClient::with_base_urls("proj", "web-api-key", &base, &base, &base).unwrap();
    client.sign_in("admin@cinematex.fr", "pw").await.unwrap();

    // Should return the cached token without hitting the mock server a second time
    let token = client.valid_id_token().await.unwrap();
    assert_eq!(token, "cached-id-tok");
    assert_eq!(handle.join().unwrap().len(), 1);
}

#[tokio::test]
async fn valid_id_token_auto_refreshes_when_missing_or_expired() {
    let (base, handle) = spawn_mock_server(vec![
        // First response: short expiresIn (< 60s margin) -> immediately considered expired
        MockResponse::ok_json(json!({
            "idToken": "short-lived-tok",
            "refreshToken": "ref-1",
            "expiresIn": "10"
        })),
        // Second response: automatic refresh triggered by valid_id_token()
        MockResponse::ok_json(json!({
            "id_token": "auto-refreshed-tok",
            "refresh_token": "ref-2",
            "expires_in": "3600"
        })),
    ]);

    let mut client =
        FirebaseClient::with_base_urls("proj", "web-api-key", &base, &base, &base).unwrap();
    client.sign_in("admin@cinematex.fr", "pw").await.unwrap();

    let token = client.valid_id_token().await.unwrap();
    assert_eq!(token, "auto-refreshed-tok");
    assert_eq!(client.refresh_token.as_deref(), Some("ref-2"));

    let reqs = handle.join().unwrap();
    assert_eq!(reqs.len(), 2);
    assert!(reqs[1].starts_with("POST /v1/token?key=web-api-key HTTP/1.1"));
}
