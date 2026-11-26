use crate::errors::{AppError, Result};
use crate::models::{Episode, Media, MediaRequest, Profile};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type FirestoreFields = BTreeMap<String, FirestoreValue>;

#[derive(Debug, Deserialize, Serialize, Clone, Eq, PartialEq)]
pub struct FirestoreArrayValue {
    #[serde(default)]
    pub values: Vec<FirestoreValue>,
}
#[derive(Debug, Deserialize, Serialize, Clone, Eq, PartialEq)]
pub struct FirestoreDocument {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub fields: FirestoreFields,
}
impl FirestoreDocument {
    pub fn id(&self) -> Option<&str> {
        self.name
            .as_deref()?
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
    }

    pub fn get_string(&self, key: &str) -> Result<String> {
        self.get_optional_string(key)?
            .ok_or_else(|| AppError::Firestore(format!("champ '{key}' manquant")))
    }
    pub fn get_optional_string(&self, key: &str) -> Result<Option<String>> {
        match self.fields.get(key) {
            None | Some(FirestoreValue::NullValue(_)) => Ok(None),
            Some(FirestoreValue::StringValue(s)) => Ok(Some(s.clone())),
            Some(_) => Err(AppError::Firestore(format!(
                "champ '{key}' attendu en string"
            ))),
        }
    }
    pub fn get_u64(&self, key: &str) -> Result<u64> {
        self.get_optional_u64(key)?
            .ok_or_else(|| AppError::Firestore(format!("champ '{key}' manquant")))
    }
    pub fn get_optional_u64(&self, key: &str) -> Result<Option<u64>> {
        match self.fields.get(key) {
            None | Some(FirestoreValue::NullValue(_)) => Ok(None),
            Some(FirestoreValue::IntegerValue(s)) => Ok(Some(s.parse::<u64>().map_err(|e| {
                AppError::Firestore(format!("champ '{key}' entier invalide ({s}) : {e}"))
            })?)),
            Some(_) => Err(AppError::Firestore(format!(
                "champ '{key}' attendu en integer"
            ))),
        }
    }
    pub fn get_bool(&self, key: &str) -> Result<bool> {
        match self.fields.get(key) {
            Some(FirestoreValue::BooleanValue(b)) => Ok(*b),
            None | Some(FirestoreValue::NullValue(_)) => {
                Err(AppError::Firestore(format!("champ '{key}' manquant")))
            }
            Some(_) => Err(AppError::Firestore(format!(
                "champ '{key}' attendu en bool"
            ))),
        }
    }
    pub fn get_string_array(&self, key: &str) -> Result<Vec<String>> {
        match self.fields.get(key) {
            None | Some(FirestoreValue::NullValue(_)) => Ok(Vec::new()),
            Some(FirestoreValue::ArrayValue(arr)) => arr
                .values
                .iter()
                .map(|v| match v {
                    FirestoreValue::StringValue(s) => Ok(s.clone()),
                    _ => Err(AppError::Firestore(format!(
                        "élément du champ '{key}' attendu en string"
                    ))),
                })
                .collect(),
            Some(_) => Err(AppError::Firestore(format!(
                "champ '{key}' attendu en tableau"
            ))),
        }
    }
    pub fn get_optional_timestamp(&self, key: &str) -> Result<Option<String>> {
        match self.fields.get(key) {
            None | Some(FirestoreValue::NullValue(_)) => Ok(None),
            Some(FirestoreValue::TimestampValue(ts)) => Ok(Some(ts.clone())),
            Some(_) => Err(AppError::Firestore(format!(
                "champ '{key}' attendu en timestamp"
            ))),
        }
    }
}
#[derive(Debug, Deserialize, Serialize, Clone, Eq, PartialEq)]
pub struct ListDocumentsResponse {
    #[serde(default)]
    pub documents: Vec<FirestoreDocument>,
}

#[expect(clippy::enum_variant_names)]
#[derive(Debug, Deserialize, Serialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FirestoreValue {
    StringValue(String),
    IntegerValue(String),
    BooleanValue(bool),
    TimestampValue(String),
    ArrayValue(FirestoreArrayValue),
    NullValue(Option<()>),
}
impl FirestoreValue {
    pub fn string(v: impl Into<String>) -> Self {
        FirestoreValue::StringValue(v.into())
    }
    pub fn integer(v: u64) -> Self {
        FirestoreValue::IntegerValue(v.to_string())
    }
    pub fn boolean(v: bool) -> Self {
        FirestoreValue::BooleanValue(v)
    }
    pub fn timestamp(v: impl Into<String>) -> Self {
        FirestoreValue::TimestampValue(v.into())
    }
    pub fn string_array(values: Vec<String>) -> Self {
        Self::ArrayValue(FirestoreArrayValue {
            values: values.into_iter().map(Self::StringValue).collect(),
        })
    }
}
pub trait FirestoreModel: Sized {
    fn from_document(doc: &FirestoreDocument) -> Result<Self>;
    fn to_fields(&self) -> FirestoreFields;
}
impl FirestoreModel for Profile {
    fn from_document(doc: &FirestoreDocument) -> Result<Self> {
        Ok(Self {
            id: doc.id().map(str::to_owned),
            name: doc.get_string("name")?,
            avatar_url: doc.get_string("avatar_url")?,
            is_kid: doc.get_bool("is_kid")?,
        })
    }
    fn to_fields(&self) -> FirestoreFields {
        let mut fields = FirestoreFields::new();
        fields.insert(
            "name".to_string(),
            FirestoreValue::StringValue(self.name.clone()),
        );
        fields.insert(
            "avatar_url".to_string(),
            FirestoreValue::StringValue(self.avatar_url.clone()),
        );
        fields.insert(
            "is_kid".to_string(),
            FirestoreValue::BooleanValue(self.is_kid),
        );
        fields
    }
}
impl FirestoreModel for Episode {
    fn from_document(doc: &FirestoreDocument) -> Result<Self> {
        Ok(Self {
            id: doc.id().map(str::to_owned),
            season: doc.get_u64("season")?,
            episode: doc.get_u64("episode")?,
            title: doc.get_string("title")?,
            file_path: doc.get_string("file_path")?,
        })
    }
    fn to_fields(&self) -> FirestoreFields {
        let mut fields = FirestoreFields::new();
        fields.insert(
            "title".to_string(),
            FirestoreValue::StringValue(self.title.clone()),
        );
        fields.insert(
            "file_path".to_string(),
            FirestoreValue::StringValue(self.file_path.clone()),
        );
        fields.insert(
            "episode".to_string(),
            FirestoreValue::IntegerValue(self.episode.to_string()),
        );
        fields.insert(
            "season".to_string(),
            FirestoreValue::IntegerValue(self.season.to_string()),
        );
        fields
    }
}
impl FirestoreModel for Media {
    fn from_document(doc: &FirestoreDocument) -> Result<Self> {
        Ok(Self {
            id: doc.id().map(str::to_owned),
            title: doc.get_string("title")?,
            tmdb_id: doc.get_u64("tmdb_id")?,
            overview: doc.get_string("overview")?,
            backdrop_url: doc.get_string("backdrop_url")?,
            poster_url: doc.get_string("poster_url")?,
            allowed_profiles: doc.get_string_array("allowed_profiles")?,
            file_path: doc.get_optional_string("file_path")?,
            media_type: doc.get_string("type")?.parse()?,
        })
    }
    fn to_fields(&self) -> FirestoreFields {
        let mut fields = FirestoreFields::new();
        fields.insert(
            "type".to_string(),
            FirestoreValue::string(self.media_type.to_string()),
        );
        fields.insert(
            "title".to_string(),
            FirestoreValue::string(self.title.clone()),
        );
        fields.insert(
            "overview".to_string(),
            FirestoreValue::string(self.overview.clone()),
        );
        fields.insert(
            "poster_url".to_string(),
            FirestoreValue::string(self.poster_url.clone()),
        );
        fields.insert(
            "backdrop_url".to_string(),
            FirestoreValue::string(self.backdrop_url.clone()),
        );
        fields.insert("tmdb_id".to_string(), FirestoreValue::integer(self.tmdb_id));
        fields.insert(
            "allowed_profiles".to_string(),
            FirestoreValue::string_array(self.allowed_profiles.clone()),
        );
        if let Some(path) = &self.file_path {
            fields.insert("file_path".to_string(), FirestoreValue::string(path));
        }
        fields
    }
}
impl FirestoreModel for MediaRequest {
    fn from_document(doc: &FirestoreDocument) -> Result<Self> {
        Ok(Self {
            id: doc.id().map(str::to_owned),
            title: doc.get_string("title")?,
            tmdb_id: doc.get_optional_u64("tmdb_id")?,
            poster_url: doc.get_optional_string("poster_url")?,
            status: doc.get_string("status")?.parse()?,
            media_type: doc.get_string("type")?.parse()?,
            profile_id: doc.get_string("profile_id")?,
            created_at: doc.get_optional_timestamp("created_at")?,
        })
    }
    fn to_fields(&self) -> FirestoreFields {
        let mut fields = FirestoreFields::new();
        fields.insert(
            "title".to_string(),
            FirestoreValue::string(self.title.to_string()),
        );
        fields.insert(
            "status".to_string(),
            FirestoreValue::string(self.status.as_str()),
        );
        fields.insert(
            "type".to_string(),
            FirestoreValue::string(self.media_type.as_str()),
        );
        fields.insert(
            "profile_id".to_string(),
            FirestoreValue::string(self.profile_id.to_string()),
        );
        if let Some(id) = self.tmdb_id {
            fields.insert("tmdb_id".to_string(), FirestoreValue::integer(id));
        }
        if let Some(url) = &self.poster_url {
            fields.insert("poster_url".to_string(), FirestoreValue::string(url));
        }
        if let Some(date) = &self.created_at {
            fields.insert("created_at".to_string(), FirestoreValue::timestamp(date));
        }

        fields
    }
}

#[cfg(test)]
#[path = "../tests/firestore.rs"]
mod tests;
