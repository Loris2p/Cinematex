use super::*;
use crate::models::{MediaType, RequestStatus};
use serde_json::json;

fn doc_with_name(name: &str, fields: FirestoreFields) -> FirestoreDocument {
    FirestoreDocument {
        name: Some(name.to_string()),
        fields,
    }
}

#[test]
fn document_id_extracts_last_path_segment_and_handles_edge_cases() {
    let doc = doc_with_name(
        "projects/cinematex/databases/(default)/documents/media/film-42",
        FirestoreFields::new(),
    );
    assert_eq!(doc.id(), Some("film-42"));

    let no_name = FirestoreDocument {
        name: None,
        fields: FirestoreFields::new(),
    };
    assert_eq!(no_name.id(), None);

    let trailing_slash = doc_with_name(
        "projects/cinematex/databases/(default)/documents/media/",
        FirestoreFields::new(),
    );
    assert_eq!(trailing_slash.id(), None);
}

#[test]
fn firestore_value_json_wire_format_matches_rest_api() {
    assert_eq!(
        serde_json::to_value(FirestoreValue::string("hello")).unwrap(),
        json!({ "stringValue": "hello" })
    );
    assert_eq!(
        serde_json::to_value(FirestoreValue::integer(42)).unwrap(),
        json!({ "integerValue": "42" })
    );
    assert_eq!(
        serde_json::to_value(FirestoreValue::boolean(true)).unwrap(),
        json!({ "booleanValue": true })
    );
    assert_eq!(
        serde_json::to_value(FirestoreValue::timestamp("2026-11-26T18:00:00Z")).unwrap(),
        json!({ "timestampValue": "2026-11-26T18:00:00Z" })
    );
    assert_eq!(
        serde_json::to_value(FirestoreValue::string_array(vec![
            "p1".to_string(),
            "p2".to_string(),
        ]))
        .unwrap(),
        json!({
            "arrayValue": {
                "values": [
                    { "stringValue": "p1" },
                    { "stringValue": "p2" }
                ]
            }
        })
    );
}

#[test]
fn list_documents_response_deserializes_present_and_empty_payloads() {
    let empty: ListDocumentsResponse = serde_json::from_value(json!({})).unwrap();
    assert!(empty.documents.is_empty());

    let parsed: ListDocumentsResponse = serde_json::from_value(json!({
        "documents": [
            {
                "name": "projects/p/databases/(default)/documents/profiles/kid1",
                "fields": {
                    "name": { "stringValue": "Léo" },
                    "avatar_url": { "stringValue": "https://img/leo.png" },
                    "is_kid": { "booleanValue": true }
                }
            }
        ]
    }))
    .unwrap();
    assert_eq!(parsed.documents.len(), 1);
    let profile = Profile::from_document(&parsed.documents[0]).unwrap();
    assert_eq!(profile.id.as_deref(), Some("kid1"));
    assert_eq!(profile.name, "Léo");
    assert!(profile.is_kid);
}

#[test]
fn profile_firestore_roundtrip_excludes_id_from_fields() {
    let profile = Profile {
        id: Some("p-admin".to_string()),
        name: "Loris".to_string(),
        avatar_url: "https://example.com/avatar.png".to_string(),
        is_kid: false,
    };

    let fields = profile.to_fields();
    assert!(!fields.contains_key("id"));

    let raw = serde_json::to_value(doc_with_name(
        "projects/cinematex/databases/(default)/documents/profiles/p-admin",
        fields,
    ))
    .unwrap();
    let doc: FirestoreDocument = serde_json::from_value(raw).unwrap();
    let decoded = Profile::from_document(&doc).unwrap();
    assert_eq!(decoded, profile);
}

#[test]
fn episode_firestore_roundtrip() {
    let ep = Episode {
        id: Some("s01e02".to_string()),
        season: 1,
        episode: 2,
        title: "Cat's in the Bag...".to_string(),
        file_path: "/Disque/Series/BB/S01E02.mkv".to_string(),
    };

    let fields = ep.to_fields();
    assert!(!fields.contains_key("id"));

    let raw = serde_json::to_value(doc_with_name(
        "projects/cinematex/databases/(default)/documents/media/bb/episodes/s01e02",
        fields,
    ))
    .unwrap();
    let doc: FirestoreDocument = serde_json::from_value(raw).unwrap();
    assert_eq!(Episode::from_document(&doc).unwrap(), ep);
}

#[test]
fn media_firestore_roundtrip_for_movie_and_series() {
    let movie = Media {
        id: Some("inception".to_string()),
        media_type: MediaType::Movie,
        title: "Inception".to_string(),
        tmdb_id: 27205,
        overview: "Rêves dans des rêves".to_string(),
        poster_url: "https://tmdb/poster.jpg".to_string(),
        backdrop_url: "https://tmdb/backdrop.jpg".to_string(),
        allowed_profiles: vec!["p1".to_string(), "p2".to_string()],
        file_path: Some("/Disque/Films/Inception.mkv".to_string()),
    };

    let movie_fields = movie.to_fields();
    assert!(!movie_fields.contains_key("id"));
    assert_eq!(
        movie_fields.get("type"),
        Some(&FirestoreValue::string("movie"))
    );
    assert!(movie_fields.contains_key("file_path"));

    let movie_doc: FirestoreDocument = serde_json::from_value(
        serde_json::to_value(doc_with_name(
            "projects/p/databases/(default)/documents/media/inception",
            movie_fields,
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(Media::from_document(&movie_doc).unwrap(), movie);

    let series = Media {
        id: Some("bb".to_string()),
        media_type: MediaType::Series,
        title: "Breaking Bad".to_string(),
        tmdb_id: 1396,
        overview: "Prof de chimie".to_string(),
        poster_url: "https://tmdb/bb.jpg".to_string(),
        backdrop_url: "https://tmdb/bb_bg.jpg".to_string(),
        allowed_profiles: vec![],
        file_path: None,
    };

    let series_fields = series.to_fields();
    assert!(!series_fields.contains_key("file_path"));
    let series_doc: FirestoreDocument = serde_json::from_value(
        serde_json::to_value(doc_with_name(
            "projects/p/databases/(default)/documents/media/bb",
            series_fields,
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(Media::from_document(&series_doc).unwrap(), series);
}

#[test]
fn media_request_firestore_roundtrip_full_and_minimal() {
    let full = MediaRequest {
        id: Some("req-1".to_string()),
        profile_id: "p1".to_string(),
        tmdb_id: Some(157336),
        media_type: MediaType::Movie,
        title: "Interstellar".to_string(),
        poster_url: Some("https://tmdb/interstellar.jpg".to_string()),
        status: RequestStatus::Pending,
        created_at: Some("2026-11-26T19:00:00Z".to_string()),
    };

    let full_fields = full.to_fields();
    assert!(!full_fields.contains_key("id"));
    assert_eq!(
        full_fields.get("type"),
        Some(&FirestoreValue::string("movie"))
    );
    assert_eq!(
        full_fields.get("created_at"),
        Some(&FirestoreValue::timestamp("2026-11-26T19:00:00Z"))
    );

    let full_doc: FirestoreDocument = serde_json::from_value(
        serde_json::to_value(doc_with_name(
            "projects/p/databases/(default)/documents/requests/req-1",
            full_fields,
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(MediaRequest::from_document(&full_doc).unwrap(), full);

    let minimal = MediaRequest {
        id: Some("req-2".to_string()),
        profile_id: "p2".to_string(),
        tmdb_id: None,
        media_type: MediaType::Series,
        title: "Andor".to_string(),
        poster_url: None,
        status: RequestStatus::Available,
        created_at: None,
    };

    let min_fields = minimal.to_fields();
    assert!(!min_fields.contains_key("tmdb_id"));
    assert!(!min_fields.contains_key("poster_url"));
    assert!(!min_fields.contains_key("created_at"));

    let min_doc: FirestoreDocument = serde_json::from_value(
        serde_json::to_value(doc_with_name(
            "projects/p/databases/(default)/documents/requests/req-2",
            min_fields,
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(MediaRequest::from_document(&min_doc).unwrap(), minimal);
}

#[test]
fn document_getters_handle_null_missing_and_type_mismatches() {
    let doc: FirestoreDocument = serde_json::from_value(json!({
        "fields": {
            "null_field": { "nullValue": null },
            "bad_int": { "integerValue": "abc" },
            "bool_field": { "booleanValue": true },
            "str_field": { "stringValue": "hello" },
            "mixed_array": {
                "arrayValue": {
                    "values": [
                        { "stringValue": "ok" },
                        { "integerValue": "123" }
                    ]
                }
            }
        }
    }))
    .unwrap();

    // Null / missing values on optional getters
    assert_eq!(doc.get_optional_string("null_field").unwrap(), None);
    assert_eq!(doc.get_optional_string("absent").unwrap(), None);
    assert_eq!(doc.get_optional_u64("null_field").unwrap(), None);
    assert_eq!(doc.get_optional_timestamp("null_field").unwrap(), None);
    assert!(doc.get_string_array("null_field").unwrap().is_empty());
    assert!(doc.get_string_array("absent").unwrap().is_empty());

    // Missing / null on required getters
    assert!(matches!(
        doc.get_string("absent").unwrap_err(),
        AppError::Firestore(msg) if msg.contains("manquant")
    ));
    assert!(matches!(
        doc.get_u64("null_field").unwrap_err(),
        AppError::Firestore(msg) if msg.contains("manquant")
    ));
    assert!(matches!(
        doc.get_bool("null_field").unwrap_err(),
        AppError::Firestore(msg) if msg.contains("manquant")
    ));

    // Type mismatches & invalid integer
    assert!(matches!(
        doc.get_string("bool_field").unwrap_err(),
        AppError::Firestore(msg) if msg.contains("string")
    ));
    assert!(matches!(
        doc.get_u64("str_field").unwrap_err(),
        AppError::Firestore(msg) if msg.contains("integer")
    ));
    assert!(matches!(
        doc.get_u64("bad_int").unwrap_err(),
        AppError::Firestore(msg) if msg.contains("invalide")
    ));
    assert!(matches!(
        doc.get_bool("str_field").unwrap_err(),
        AppError::Firestore(msg) if msg.contains("bool")
    ));
    assert!(matches!(
        doc.get_string_array("str_field").unwrap_err(),
        AppError::Firestore(msg) if msg.contains("tableau")
    ));
    assert!(matches!(
        doc.get_string_array("mixed_array").unwrap_err(),
        AppError::Firestore(msg) if msg.contains("string")
    ));
    assert!(matches!(
        doc.get_optional_timestamp("str_field").unwrap_err(),
        AppError::Firestore(msg) if msg.contains("timestamp")
    ));
}

#[test]
fn from_document_rejects_unknown_media_type_and_request_status() {
    let mut media_fields = Media {
        id: None,
        media_type: MediaType::Movie,
        title: "Test".to_string(),
        tmdb_id: 1,
        overview: String::new(),
        poster_url: String::new(),
        backdrop_url: String::new(),
        allowed_profiles: vec![],
        file_path: None,
    }
    .to_fields();
    media_fields.insert("type".to_string(), FirestoreValue::string("podcast"));
    let err = Media::from_document(&doc_with_name("media/1", media_fields)).unwrap_err();
    assert!(matches!(err, AppError::Firestore(msg) if msg.contains("podcast")));

    let mut req_fields = MediaRequest {
        id: None,
        profile_id: "p1".to_string(),
        tmdb_id: None,
        media_type: MediaType::Movie,
        title: "Test".to_string(),
        poster_url: None,
        status: RequestStatus::Pending,
        created_at: None,
    }
    .to_fields();
    req_fields.insert("status".to_string(), FirestoreValue::string("archived"));
    let err = MediaRequest::from_document(&doc_with_name("requests/1", req_fields)).unwrap_err();
    assert!(matches!(err, AppError::Firestore(msg) if msg.contains("archived")));
}
