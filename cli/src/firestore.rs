use crate::errors::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub(crate) type FirestoreFields = BTreeMap<String, FirestoreValue>;

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
