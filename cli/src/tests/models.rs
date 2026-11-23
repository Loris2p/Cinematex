use super::*;
use serde_json::json;

#[test]
fn media_type_str_display_and_from_str() {
    assert_eq!(MediaType::Movie.as_str(), "movie");
    assert_eq!(MediaType::Series.as_str(), "series");
    assert_eq!(MediaType::Movie.to_string(), "movie");
    assert_eq!(MediaType::Series.to_string(), "series");

    assert_eq!("movie".parse::<MediaType>().unwrap(), MediaType::Movie);
    assert_eq!("series".parse::<MediaType>().unwrap(), MediaType::Series);

    let err = "documentary".parse::<MediaType>().unwrap_err();
    assert!(matches!(err, AppError::Firestore(msg) if msg.contains("documentary")));
}

#[test]
fn media_type_serde_roundtrip() {
    assert_eq!(
        serde_json::to_value(MediaType::Movie).unwrap(),
        json!("movie")
    );
    assert_eq!(
        serde_json::to_value(MediaType::Series).unwrap(),
        json!("series")
    );
    assert_eq!(
        serde_json::from_value::<MediaType>(json!("movie")).unwrap(),
        MediaType::Movie
    );
    assert_eq!(
        serde_json::from_value::<MediaType>(json!("series")).unwrap(),
        MediaType::Series
    );
}

#[test]
fn request_status_str_display_and_from_str() {
    let cases = [
        (RequestStatus::Pending, "pending"),
        (RequestStatus::Approved, "approved"),
        (RequestStatus::Available, "available"),
        (RequestStatus::Rejected, "rejected"),
    ];
    for (variant, expected) in cases {
        assert_eq!(variant.as_str(), expected);
        assert_eq!(variant.to_string(), expected);
        assert_eq!(expected.parse::<RequestStatus>().unwrap(), variant);
    }

    let err = "unknown".parse::<RequestStatus>().unwrap_err();
    assert!(matches!(err, AppError::Firestore(msg) if msg.contains("unknown")));
}

#[test]
fn request_status_serde_roundtrip() {
    for (variant, expected) in [
        (RequestStatus::Pending, "pending"),
        (RequestStatus::Approved, "approved"),
        (RequestStatus::Available, "available"),
        (RequestStatus::Rejected, "rejected"),
    ] {
        assert_eq!(serde_json::to_value(variant).unwrap(), json!(expected));
        assert_eq!(
            serde_json::from_value::<RequestStatus>(json!(expected)).unwrap(),
            variant
        );
    }
}

#[test]
fn media_serializes_type_field_and_optional_file_path() {
    let movie = Media {
        id: Some("doc-1".to_string()),
        media_type: MediaType::Movie,
        title: "Inception".to_string(),
        tmdb_id: 27205,
        overview: "Un voleur...".to_string(),
        poster_url: "https://image.tmdb.org/t/p/w500/poster.jpg".to_string(),
        backdrop_url: "https://image.tmdb.org/t/p/w1280/backdrop.jpg".to_string(),
        allowed_profiles: vec!["p1".to_string(), "p2".to_string()],
        file_path: Some("/Disque dur/Films/Inception.mkv".to_string()),
    };

    let val = serde_json::to_value(&movie).unwrap();
    assert_eq!(val["type"], json!("movie"));
    assert_eq!(val["file_path"], json!("/Disque dur/Films/Inception.mkv"));
    assert_eq!(serde_json::from_value::<Media>(val).unwrap(), movie);

    let series = Media {
        id: None,
        media_type: MediaType::Series,
        file_path: None,
        ..movie
    };
    let val = serde_json::to_value(&series).unwrap();
    assert_eq!(val["type"], json!("series"));
    assert!(val["file_path"].is_null());
    assert_eq!(serde_json::from_value::<Media>(val).unwrap(), series);
}

#[test]
fn profile_episode_and_request_serde_roundtrip() {
    let profile = Profile {
        id: Some("prof-1".to_string()),
        name: "Loris".to_string(),
        avatar_url: "https://example.com/a.png".to_string(),
        is_kid: false,
    };
    let roundtrip: Profile =
        serde_json::from_value(serde_json::to_value(&profile).unwrap()).unwrap();
    assert_eq!(roundtrip, profile);

    let ep = Episode {
        id: None,
        season: 1,
        episode: 2,
        title: "Pilot".to_string(),
        file_path: "/Disque dur/Series/S01E02.mkv".to_string(),
    };
    let roundtrip: Episode = serde_json::from_value(serde_json::to_value(&ep).unwrap()).unwrap();
    assert_eq!(roundtrip, ep);

    let req = MediaRequest {
        id: Some("req-1".to_string()),
        profile_id: "prof-1".to_string(),
        tmdb_id: None,
        media_type: MediaType::Movie,
        title: "Interstellar".to_string(),
        poster_url: None,
        status: RequestStatus::Pending,
        created_at: Some("2026-11-23T20:00:00Z".to_string()),
    };
    let val = serde_json::to_value(&req).unwrap();
    assert_eq!(val["type"], json!("movie"));
    assert_eq!(val["status"], json!("pending"));
    assert_eq!(serde_json::from_value::<MediaRequest>(val).unwrap(), req);
}
