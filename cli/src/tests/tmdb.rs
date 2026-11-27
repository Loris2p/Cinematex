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

fn spawn_mock_server(responses: Vec<MockResponse>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let addr = listener.local_addr().expect("local addr");
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

fn test_tmdb_client(token: &str, base_url: &str) -> TmdbClient {
    let settings = TmdbSettings {
        base_url: base_url.to_string(),
        image_base_url: "https://image.tmdb.org/t/p".to_string(),
        language: "fr-FR".to_string(),
    };
    let http_cfg = HttpSettings {
        connect_timeout_secs: 5,
        request_timeout_secs: 15,
    };
    TmdbClient::new(token, &settings, &http_cfg).expect("client")
}

#[test]
fn image_url_helpers_format_paths_and_handle_empty() {
    let client = test_tmdb_client("tok", "https://api.themoviedb.org/3");
    assert_eq!(
        client.poster_url(Some("/poster.jpg")),
        "https://image.tmdb.org/t/p/w500/poster.jpg"
    );
    assert_eq!(
        client.backdrop_url(Some("/backdrop.jpg")),
        "https://image.tmdb.org/t/p/w1280/backdrop.jpg"
    );
    assert_eq!(client.poster_url(Some("")), "");
    assert_eq!(client.poster_url(None), "");
    assert_eq!(client.backdrop_url(Some("")), "");
    assert_eq!(client.backdrop_url(None), "");
}

#[tokio::test]
async fn search_movie_with_year_sends_bearer_and_query_params() {
    let body = json!({
        "results": [
            {
                "id": 27205,
                "title": "Inception",
                "overview": "Un voleur qui s'infiltre dans les rêves.",
                "poster_path": "/inception_poster.jpg",
                "backdrop_path": "/inception_backdrop.jpg",
                "release_date": "2010-07-16"
            }
        ]
    });
    let (base_url, handle) = spawn_mock_server(vec![MockResponse::ok_json(body)]);
    let client = test_tmdb_client("fake-tmdb-token", &base_url);

    let movies = client
        .search_movie("Inception", Some(2010))
        .await
        .expect("search_movie");

    assert_eq!(movies.len(), 1);
    assert_eq!(movies[0].id, 27205);
    assert_eq!(movies[0].title, "Inception");
    assert_eq!(
        movies[0].poster_path.as_deref(),
        Some("/inception_poster.jpg")
    );
    assert_eq!(
        movies[0].backdrop_path.as_deref(),
        Some("/inception_backdrop.jpg")
    );
    assert_eq!(movies[0].release_date.as_deref(), Some("2010-07-16"));

    let requests = handle.join().expect("join");
    let req = &requests[0];
    assert!(req.starts_with("GET /search/movie?"));
    assert!(req.contains("query=Inception"));
    assert!(req.contains("language=fr-FR"));
    assert!(req.contains("year=2010"));
    assert!(
        req.to_ascii_lowercase()
            .contains("authorization: bearer fake-tmdb-token")
    );
}

#[tokio::test]
async fn search_movie_without_year_omits_year_param() {
    let body = json!({
        "results": [
            {
                "id": 603,
                "title": "Matrix",
                "poster_path": null,
                "backdrop_path": null,
                "release_date": null
            }
        ]
    });
    let (base_url, handle) = spawn_mock_server(vec![MockResponse::ok_json(body)]);
    let client = test_tmdb_client("tok", &base_url);

    let movies = client
        .search_movie("Matrix", None)
        .await
        .expect("search_movie");

    assert_eq!(movies.len(), 1);
    assert_eq!(movies[0].id, 603);
    assert_eq!(movies[0].overview, "");
    assert_eq!(movies[0].poster_path, None);

    let requests = handle.join().expect("join");
    let req = &requests[0];
    assert!(req.starts_with("GET /search/movie?"));
    assert!(req.contains("query=Matrix"));
    assert!(req.contains("language=fr-FR"));
    assert!(!req.contains("year="));
}

#[tokio::test]
async fn search_tv_with_year_uses_first_air_date_year() {
    let body = json!({
        "results": [
            {
                "id": 1396,
                "name": "Breaking Bad",
                "overview": "Un professeur de chimie...",
                "poster_path": "/bb.jpg",
                "backdrop_path": "/bb_bg.jpg",
                "first_air_date": "2008-01-20"
            }
        ]
    });
    let (base_url, handle) = spawn_mock_server(vec![MockResponse::ok_json(body)]);
    let client = test_tmdb_client("tok", &base_url);

    let shows = client
        .search_tv("Breaking Bad", Some(2008))
        .await
        .expect("search_tv");

    assert_eq!(shows.len(), 1);
    assert_eq!(shows[0].id, 1396);
    assert_eq!(shows[0].name, "Breaking Bad");
    assert_eq!(shows[0].first_air_date.as_deref(), Some("2008-01-20"));

    let requests = handle.join().expect("join");
    let req = &requests[0];
    assert!(req.starts_with("GET /search/tv?"));
    assert!(req.contains("first_air_date_year=2008"));
    assert!(req.contains("language=fr-FR"));
}

#[tokio::test]
async fn get_season_deserializes_episodes() {
    let body = json!({
        "season_number": 1,
        "name": "Saison 1",
        "episodes": [
            {
                "season_number": 1,
                "episode_number": 1,
                "name": "Chute libre",
                "overview": "Premier épisode."
            },
            {
                "season_number": 1,
                "episode_number": 2,
                "name": "Le Choix"
            }
        ]
    });
    let (base_url, handle) = spawn_mock_server(vec![MockResponse::ok_json(body)]);
    let client = test_tmdb_client("tok", &base_url);

    let season = client.get_season(1396, 1).await.expect("get_season");

    assert_eq!(season.season_number, 1);
    assert_eq!(season.name, "Saison 1");
    assert_eq!(season.episodes.len(), 2);
    assert_eq!(season.episodes[0].episode_number, 1);
    assert_eq!(season.episodes[0].name, "Chute libre");
    assert_eq!(season.episodes[1].episode_number, 2);
    assert_eq!(season.episodes[1].overview, "");

    let requests = handle.join().expect("join");
    let req = &requests[0];
    assert!(req.starts_with("GET /tv/1396/season/1?"));
    assert!(req.contains("language=fr-FR"));
}

#[tokio::test]
async fn tmdb_error_json_extracts_status_message() {
    let err_body = json!({
        "status_code": 7,
        "status_message": "Invalid API key: You must be granted a valid key."
    })
    .to_string();
    let (base_url, handle) =
        spawn_mock_server(vec![MockResponse::status("401 Unauthorized", &err_body)]);
    let client = test_tmdb_client("bad-token", &base_url);

    let err = client
        .search_movie("Inception", None)
        .await
        .expect_err("should fail with 401");

    match err {
        AppError::Tmdb(msg) => {
            assert!(msg.contains("401"), "msg={msg}");
            assert!(msg.contains("Invalid API key"), "msg={msg}");
        }
        other => panic!("expected AppError::Tmdb, got {other:?}"),
    }
    let _ = handle.join();
}

#[tokio::test]
async fn tmdb_error_plain_text_falls_back_to_body() {
    let (base_url, handle) = spawn_mock_server(vec![MockResponse::status(
        "500 Internal Server Error",
        "upstream gateway down",
    )]);
    let client = test_tmdb_client("tok", &base_url);

    let err = client
        .get_season(1396, 1)
        .await
        .expect_err("should fail with 500");

    match err {
        AppError::Tmdb(msg) => {
            assert!(msg.contains("500"), "msg={msg}");
            assert!(msg.contains("upstream gateway down"), "msg={msg}");
        }
        other => panic!("expected AppError::Tmdb, got {other:?}"),
    }
    let _ = handle.join();
}
