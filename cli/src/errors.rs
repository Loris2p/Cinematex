pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("erreur d'entrée sortie : {0}")]
    Io(#[from] std::io::Error),
    #[error("erreur réseau : {0}")]
    Http(#[from] reqwest::Error),
    #[error("erreur de sérialisation json : {0}")]
    Json(#[from] serde_json::Error),
    #[error("erreur d'expression rationnelle : {0}")]
    Regex(#[from] regex::Error),
    #[error("erreur de décodage base64 : {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("erreur de décodage hexadécimal : {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("erreur de configuration : {0}")]
    Config(String),
    #[error("erreur d'authentification freebox : {0}")]
    FreeboxAuth(String),
    #[error("erreur api freebox : {0}")]
    FreeboxApi(String),
    #[error("erreur d'authentification firebase : {0}")]
    FirebaseAuth(String),
    #[error("erreur firestore : {0}")]
    Firestore(String),
    #[error("erreur tmdb : {0}")]
    Tmdb(String),
}
