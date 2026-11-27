use std::fs;
const ALLOWED_KEYS: &[&str] = &[
    "FIREBASE_PROJECT_ID",
    "FIREBASE_WEB_API_KEY",
    "TMDB_READING_TOKEN",
];
fn main() {
    println!("cargo::rerun-if-changed=../.env");

    if let Ok(content) = fs::read_to_string("../.env") {
        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let Some((raw_key, raw_value)) = line.split_once('=') else {
                continue;
            };

            let key = raw_key.trim();
            let value = raw_value.trim().trim_matches('"').trim_matches('\'');

            if ALLOWED_KEYS.contains(&key) && !value.is_empty() {
                println!("cargo::rustc-env={key}={value}");
            }
        }
    }
}
