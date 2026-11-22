use super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD};

#[test]
fn formats_domain_error_variants() {
    assert_eq!(
        AppError::Config("clé manquante".to_string()).to_string(),
        "erreur de configuration : clé manquante"
    );
    assert_eq!(
        AppError::FreeboxAuth("refusé".to_string()).to_string(),
        "erreur d'authentification freebox : refusé"
    );
    assert_eq!(
        AppError::FreeboxApi("invalid_token".to_string()).to_string(),
        "erreur api freebox : invalid_token"
    );
    assert_eq!(
        AppError::FirebaseAuth("expired".to_string()).to_string(),
        "erreur d'authentification firebase : expired"
    );
    assert_eq!(
        AppError::Firestore("not found".to_string()).to_string(),
        "erreur firestore : not found"
    );
    assert_eq!(
        AppError::Tmdb("rate limited".to_string()).to_string(),
        "erreur tmdb : rate limited"
    );
}

#[test]
fn converts_external_errors_via_from() {
    let io_err: AppError = std::io::Error::new(std::io::ErrorKind::NotFound, "absent").into();
    assert!(matches!(io_err, AppError::Io(_)));
    assert!(io_err.to_string().contains("entrée sortie"));

    let json_err: AppError = serde_json::from_str::<serde_json::Value>("{").unwrap_err().into();
    assert!(matches!(json_err, AppError::Json(_)));
    assert!(json_err.to_string().contains("json"));

    #[allow(clippy::invalid_regex)]
    let regex_err: AppError = regex::Regex::new("[").unwrap_err().into();
    assert!(matches!(regex_err, AppError::Regex(_)));
    assert!(regex_err.to_string().contains("expression rationnelle"));

    let b64_err: AppError = STANDARD.decode("!!!").unwrap_err().into();
    assert!(matches!(b64_err, AppError::Base64(_)));
    assert!(b64_err.to_string().contains("base64"));

    let hex_err: AppError = hex::decode("zz").unwrap_err().into();
    assert!(matches!(hex_err, AppError::Hex(_)));
    assert!(hex_err.to_string().contains("hexadécimal"));
}
