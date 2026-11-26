use super::firestore::{FirestoreDocument, FirestoreFields, FirestoreModel, ListDocumentsResponse};
use super::{FirebaseClient, parse_firebase_response};
use crate::errors::{AppError, Result};
use reqwest::StatusCode;
impl FirebaseClient {
    fn firestore_url(&self, path: &str) -> String {
        let route = path.trim_start_matches('/');
        format!(
            "{}/v1/projects/{}/databases/(default)/documents/{}",
            self.firestore_base_url, self.project_id, route
        )
    }
    pub async fn list_documents<T: FirestoreModel>(
        &mut self,
        collection_path: &str,
    ) -> Result<Vec<T>> {
        let token = self.valid_id_token().await?.to_owned();
        let resp = self
            .http
            .get(self.firestore_url(collection_path))
            .bearer_auth(token)
            .send()
            .await?;
        let data =
            parse_firebase_response::<ListDocumentsResponse>(resp, AppError::Firestore).await?;
        data.documents.iter().map(T::from_document).collect()
    }
    pub async fn get_document<T: FirestoreModel>(&mut self, doc_path: &str) -> Result<Option<T>> {
        let token = self.valid_id_token().await?.to_owned();
        let resp = self
            .http
            .get(self.firestore_url(doc_path))
            .bearer_auth(token)
            .send()
            .await?;
        if resp.status() == StatusCode::NOT_FOUND {
            Ok(None)
        } else {
            let data =
                parse_firebase_response::<FirestoreDocument>(resp, AppError::Firestore).await?;
            Ok(Some(T::from_document(&data)?))
        }
    }
    pub async fn create_or_update_document<T: FirestoreModel>(
        &mut self,
        collection_path: &str,
        doc_id: Option<&str>,
        model: &T,
    ) -> Result<T> {
        let token = self.valid_id_token().await?.to_owned();
        let body = FirestoreDocument {
            name: None,
            fields: model.to_fields(),
        };
        let res = match doc_id {
            Some(id) => {
                let doc_path = format!("{}/{}", collection_path.trim_end_matches('/'), id);
                self.http
                    .patch(self.firestore_url(&doc_path))
                    .bearer_auth(&token)
                    .json(&body)
                    .send()
                    .await?
            }
            None => {
                self.http
                    .post(self.firestore_url(collection_path))
                    .bearer_auth(&token)
                    .json(&body)
                    .send()
                    .await?
            }
        };
        let doc = parse_firebase_response::<FirestoreDocument>(res, AppError::Firestore).await?;
        T::from_document(&doc)
    }
    pub async fn patch_fields(
        &mut self,
        doc_path: &str,
        fields: FirestoreFields,
    ) -> Result<FirestoreDocument> {
        let token = self.valid_id_token().await?.to_owned();
        let body = FirestoreDocument { name: None, fields };
        let mask: Vec<(&str, &str)> = body
            .fields
            .keys()
            .map(|k| ("updateMask.fieldPaths", k.as_str()))
            .collect();
        let res = self
            .http
            .patch(self.firestore_url(doc_path))
            .bearer_auth(&token)
            .query(&mask)
            .json(&body)
            .send()
            .await?;
        parse_firebase_response::<FirestoreDocument>(res, AppError::Firestore).await
    }
}

#[cfg(test)]
#[path = "../tests/firestore_client.rs"]
mod tests;
