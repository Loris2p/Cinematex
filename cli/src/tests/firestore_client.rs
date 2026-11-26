use super::*;
use crate::firebase::firestore::FirestoreValue;
use crate::models::Profile;
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use tokio::time::{Duration, Instant};

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

fn authenticated_client(firestore_base: &str) -> FirebaseClient {
    let mut client = FirebaseClient::with_base_urls(
        "cinematex-test",
        "fake-api-key",
        "http://127.0.0.1:1",
        "http://127.0.0.1:1",
        firestore_base,
    )
    .unwrap();
    client.id_token = Some("cached-id-token".to_string());
    client.expires_at = Some(Instant::now() + Duration::from_secs(300));
    client
}

#[test]
fn firestore_url_trims_leading_slashes() {
    let client = authenticated_client("http://127.0.0.1:9999/");
    assert_eq!(
        client.firestore_url("media/abc"),
        "http://127.0.0.1:9999/v1/projects/cinematex-test/databases/(default)/documents/media/abc"
    );
    assert_eq!(
        client.firestore_url("///profiles"),
        "http://127.0.0.1:9999/v1/projects/cinematex-test/databases/(default)/documents/profiles"
    );
}

#[tokio::test]
async fn list_documents_sends_bearer_token_and_decodes_models() {
    let (base, handle) = spawn_mock_server(vec![MockResponse::ok_json(json!({
        "documents": [
            {
                "name": "projects/cinematex-test/databases/(default)/documents/profiles/p1",
                "fields": {
                    "name": { "stringValue": "Loris" },
                    "avatar_url": { "stringValue": "https://img/loris.png" },
                    "is_kid": { "booleanValue": false }
                }
            }
        ]
    }))]);

    let mut client = authenticated_client(&base);
    let profiles: Vec<Profile> = client.list_documents("profiles").await.unwrap();

    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].id.as_deref(), Some("p1"));
    assert_eq!(profiles[0].name, "Loris");
    assert!(!profiles[0].is_kid);

    let reqs = handle.join().unwrap();
    assert_eq!(reqs.len(), 1);
    assert!(reqs[0].starts_with(
        "GET /v1/projects/cinematex-test/databases/(default)/documents/profiles HTTP/1.1"
    ));
    assert!(
        reqs[0]
            .to_ascii_lowercase()
            .contains("authorization: bearer cached-id-token")
    );
}

#[tokio::test]
async fn get_document_returns_some_on_200_and_none_on_404() {
    let (base, handle) = spawn_mock_server(vec![
        MockResponse::ok_json(json!({
            "name": "projects/cinematex-test/databases/(default)/documents/profiles/kid1",
            "fields": {
                "name": { "stringValue": "Léo" },
                "avatar_url": { "stringValue": "https://img/leo.png" },
                "is_kid": { "booleanValue": true }
            }
        })),
        MockResponse::status(
            "404 Not Found",
            r#"{"error":{"code":404,"message":"Document not found"}}"#,
        ),
    ]);

    let mut client = authenticated_client(&base);
    let found: Option<Profile> = client.get_document("profiles/kid1").await.unwrap();
    assert_eq!(found.as_ref().and_then(|p| p.id.as_deref()), Some("kid1"));
    assert!(found.unwrap().is_kid);

    let missing: Option<Profile> = client.get_document("profiles/unknown").await.unwrap();
    assert!(missing.is_none());

    let reqs = handle.join().unwrap();
    assert_eq!(reqs.len(), 2);
    assert!(reqs[0].starts_with(
        "GET /v1/projects/cinematex-test/databases/(default)/documents/profiles/kid1 HTTP/1.1"
    ));
    assert!(reqs[1].starts_with(
        "GET /v1/projects/cinematex-test/databases/(default)/documents/profiles/unknown HTTP/1.1"
    ));
}

#[tokio::test]
async fn create_or_update_document_uses_post_without_id_and_patch_with_id() {
    let (base, handle) = spawn_mock_server(vec![
        MockResponse::ok_json(json!({
            "name": "projects/cinematex-test/databases/(default)/documents/profiles/auto-123",
            "fields": {
                "name": { "stringValue": "Invité" },
                "avatar_url": { "stringValue": "https://img/guest.png" },
                "is_kid": { "booleanValue": false }
            }
        })),
        MockResponse::ok_json(json!({
            "name": "projects/cinematex-test/databases/(default)/documents/profiles/p-admin",
            "fields": {
                "name": { "stringValue": "Loris" },
                "avatar_url": { "stringValue": "https://img/loris.png" },
                "is_kid": { "booleanValue": false }
            }
        })),
    ]);

    let mut client = authenticated_client(&base);
    let guest = Profile {
        id: None,
        name: "Invité".to_string(),
        avatar_url: "https://img/guest.png".to_string(),
        is_kid: false,
    };
    let created = client
        .create_or_update_document("profiles", None, &guest)
        .await
        .unwrap();
    assert_eq!(created.id.as_deref(), Some("auto-123"));

    let admin = Profile {
        id: Some("p-admin".to_string()),
        name: "Loris".to_string(),
        avatar_url: "https://img/loris.png".to_string(),
        is_kid: false,
    };
    let upserted = client
        .create_or_update_document("profiles/", Some("p-admin"), &admin)
        .await
        .unwrap();
    assert_eq!(upserted.id.as_deref(), Some("p-admin"));

    let reqs = handle.join().unwrap();
    assert_eq!(reqs.len(), 2);
    assert!(reqs[0].starts_with(
        "POST /v1/projects/cinematex-test/databases/(default)/documents/profiles HTTP/1.1"
    ));
    assert!(reqs[0].contains(r#""stringValue":"Invité""#));
    let post_body: serde_json::Value =
        serde_json::from_str(reqs[0].split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert!(post_body.get("name").is_none());
    assert!(post_body.get("fields").is_some());

    assert!(reqs[1].starts_with(
        "PATCH /v1/projects/cinematex-test/databases/(default)/documents/profiles/p-admin HTTP/1.1"
    ));
    assert!(reqs[1].contains(r#""stringValue":"Loris""#));
}

#[tokio::test]
async fn patch_fields_sends_update_mask_and_fields_payload() {
    let (base, handle) = spawn_mock_server(vec![MockResponse::ok_json(json!({
        "name": "projects/cinematex-test/databases/(default)/documents/requests/req-1",
        "fields": {
            "status": { "stringValue": "available" },
            "title": { "stringValue": "Interstellar" }
        }
    }))]);

    let mut client = authenticated_client(&base);
    let mut fields = FirestoreFields::new();
    fields.insert("status".to_string(), FirestoreValue::string("available"));
    fields.insert("title".to_string(), FirestoreValue::string("Interstellar"));

    let doc = client.patch_fields("requests/req-1", fields).await.unwrap();
    assert_eq!(doc.id(), Some("req-1"));
    assert_eq!(doc.get_string("status").unwrap(), "available");

    let reqs = handle.join().unwrap();
    assert_eq!(reqs.len(), 1);
    let first_line = reqs[0].lines().next().unwrap_or("");
    assert!(first_line.starts_with(
        "PATCH /v1/projects/cinematex-test/databases/(default)/documents/requests/req-1?"
    ));
    assert!(first_line.contains("updateMask.fieldPaths=status"));
    assert!(first_line.contains("updateMask.fieldPaths=title"));
    assert!(reqs[0].contains(r#""status":{"stringValue":"available"}"#));
}

#[tokio::test]
async fn firestore_http_error_maps_to_app_error_firestore() {
    let (base, handle) = spawn_mock_server(vec![MockResponse::status(
        "403 Forbidden",
        r#"{"error":{"code":403,"message":"Missing or insufficient permissions."}}"#,
    )]);

    let mut client = authenticated_client(&base);
    let err = client
        .list_documents::<Profile>("profiles")
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        AppError::Firestore(msg)
            if msg.contains("403") && msg.contains("Missing or insufficient permissions.")
    ));

    handle.join().unwrap();
}
