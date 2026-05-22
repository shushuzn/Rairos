//! Chroma vector database client

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::client::{DistanceMetric, SearchHit, VectorStore, VectorStoreConfig};
use crate::error::VectorError;

/// Chroma API client
pub struct ChromaClient {
    base_url: String,
    client: reqwest::Client,
}

impl ChromaClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_client(base_url: impl Into<String>, client: reqwest::Client) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client,
        }
    }

    async fn request<T: for<'de> Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T, VectorError> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let mut request = self.client.request(method, &url);

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| VectorError::ApiError(format!("Chroma request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(VectorError::ApiError(format!("Chroma API error {}: {}", status, body)));
        }

        response
            .json()
            .await
            .map_err(|e| VectorError::ApiError(format!("Chroma parse error: {}", e)))
    }
}

#[derive(Serialize)]
struct ChromaAddRequest {
    ids: Vec<String>,
    embeddings: Vec<Vec<f32>>,
    metadatas: Vec<serde_json::Value>,
    documents: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct ChromaQueryRequest {
    #[serde(rename = "query_embeddings")]
    query_embeddings: Vec<Vec<f32>>,
    #[serde(rename = "n_results")]
    n_results: usize,
    #[serde(rename = "where")]
    filter: Option<serde_json::Value>,
    #[serde(rename = "include")]
    include: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct ChromaCountResponse {
    #[serde(rename = "count")]
    count: usize,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ChromaQueryResponse {
    ids: Vec<Vec<String>>,
    distances: Vec<Vec<f32>>,
    embeddings: Option<Vec<Vec<Vec<f32>>>>,
    metadatas: Vec<Vec<serde_json::Value>>,
    documents: Vec<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
struct ChromaDeleteRequest {
    #[serde(rename = "where")]
    filter: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ChromaGetResponse {
    ids: Vec<String>,
    embeddings: Option<Vec<Vec<f32>>>,
    metadatas: Vec<serde_json::Value>,
    documents: Vec<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ChromaCreateRequest {
    name: String,
    #[serde(rename = "get_or_create")]
    get_or_create: bool,
    #[serde(rename = "metadata")]
    metadata: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ChromaResponse {
    #[serde(rename = "success")]
    success: bool,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ChromaError {
    #[serde(rename = "error")]
    error: String,
}

#[async_trait]
impl VectorStore for ChromaClient {
    async fn initialize(&self, config: &VectorStoreConfig) -> Result<(), VectorError> {
        let body = serde_json::json!({
            "name": config.collection_name,
            "get_or_create": true,
            "metadata": {
                "hnsw:space": match config.distance_metric {
                    DistanceMetric::Cosine => "cosine",
                    DistanceMetric::Euclidean => "l2",
                    DistanceMetric::DotProduct => "ip",
                }
            }
        });

        let _: serde_json::Value = self
            .request(reqwest::Method::POST, "/api/v1/collections", Some(body))
            .await?;

        Ok(())
    }

    async fn upsert_one(
        &self,
        id: &str,
        embedding: &[f32],
        payload: Option<serde_json::Value>,
    ) -> Result<(), VectorError> {
        let document = payload
            .as_ref()
            .and_then(|p| p.get("text").or_else(|| p.get("content")))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let metadata = payload.unwrap_or(serde_json::Value::Null);

        let body = ChromaAddRequest {
            ids: vec![id.to_string()],
            embeddings: vec![embedding.to_vec()],
            metadatas: vec![metadata],
            documents: vec![document],
        };

        let _: serde_json::Value = self
            .request(
                reqwest::Method::POST,
                &format!("/api/v1/collections/{}/add", config_collection_name()),
                Some(serde_json::to_value(body).unwrap()),
            )
            .await?;

        Ok(())
    }

    async fn upsert_batch(
        &self,
        vectors: Vec<(String, Vec<f32>, Option<serde_json::Value>)>,
    ) -> Result<(), VectorError> {
        if vectors.is_empty() {
            return Ok(());
        }

        let ids: Vec<String> = vectors.iter().map(|(id, _, _)| id.clone()).collect();
        let embeddings: Vec<Vec<f32>> = vectors.iter().map(|(_, e, _)| e.clone()).collect();
        let documents: Vec<String> = vectors
            .iter()
            .map(|(_, _, p)| {
                p.as_ref()
                    .and_then(|v| v.get("text").or_else(|| v.get("content")))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .collect();
        let metadatas: Vec<serde_json::Value> = vectors
            .iter()
            .map(|(_, _, p)| p.clone().unwrap_or(serde_json::Value::Null))
            .collect();

        let body = ChromaAddRequest {
            ids,
            embeddings,
            metadatas,
            documents,
        };

        let _: serde_json::Value = self
            .request(
                reqwest::Method::POST,
                &format!("/api/v1/collections/{}/add", config_collection_name()),
                Some(serde_json::to_value(body).unwrap()),
            )
            .await?;

        Ok(())
    }

    async fn search(
        &self,
        query: &[f32],
        top_k: usize,
        filter: Option<&str>,
    ) -> Result<Vec<SearchHit>, VectorError> {
        let filter_json: Option<serde_json::Value> = filter.and_then(|f| serde_json::from_str(f).ok());

        let body = ChromaQueryRequest {
            query_embeddings: vec![query.to_vec()],
            n_results: top_k,
            filter: filter_json,
            include: Some(vec!["metadatas".to_string(), "documents".to_string(), "distances".to_string()]),
        };

        let resp: ChromaQueryResponse = self
            .request(
                reqwest::Method::POST,
                &format!("/api/v1/collections/{}/query", config_collection_name()),
                Some(serde_json::to_value(body).unwrap()),
            )
            .await?;

        let ids = &resp.ids[0];
        let distances = &resp.distances[0];
        let metadatas = &resp.metadatas[0];
        let documents = &resp.documents[0];

        let hits: Vec<SearchHit> = ids
            .iter()
            .zip(distances.iter())
            .zip(metadatas.iter())
            .zip(documents.iter())
            .map(|(((id, dist), meta), _doc)| SearchHit {
                id: id.clone(),
                score: 1.0 - dist, // Chroma returns distance, convert to similarity
                payload: Some(meta.clone()).filter(|v| !v.is_null()),
                embedding: None,
            })
            .collect();

        Ok(hits)
    }

    async fn delete(&self, id: &str) -> Result<(), VectorError> {
        let body = ChromaDeleteRequest {
            filter: Some(serde_json::json!({ "id": id })),
        };

        let _: serde_json::Value = self
            .request(
                reqwest::Method::POST,
                &format!("/api/v1/collections/{}/delete", config_collection_name()),
                Some(serde_json::to_value(body).unwrap()),
            )
            .await?;

        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<SearchHit>, VectorError> {
        let body = serde_json::json!({
            "ids": [id],
            "include": ["metadatas", "documents", "embeddings"]
        });

        let resp: ChromaGetResponse = self
            .request(
                reqwest::Method::POST,
                &format!("/api/v1/collections/{}/get", config_collection_name()),
                Some(body),
            )
            .await?;

        if resp.ids.is_empty() {
            return Ok(None);
        }

        Ok(Some(SearchHit {
            id: resp.ids[0].clone(),
            score: 1.0,
            payload: Some(resp.metadatas[0].clone()).filter(|v| !v.is_null()),
            embedding: resp.embeddings.map(|e| e[0].clone()),
        }))
    }

    async fn count(&self) -> Result<usize, VectorError> {
        let resp: ChromaCountResponse = self
            .request(
                reqwest::Method::GET,
                &format!("/api/v1/collections/{}/count", config_collection_name()),
                None,
            )
            .await?;
        Ok(resp.count)
    }

    async fn exists(&self, id: &str) -> Result<bool, VectorError> {
        Ok(self.get(id).await?.is_some())
    }

    async fn drop_collection(&self) -> Result<(), VectorError> {
        let _: serde_json::Value = self
            .request(
                reqwest::Method::DELETE,
                &format!("/api/v1/collections/{}", config_collection_name()),
                None,
            )
            .await?;
        Ok(())
    }
}

// Thread-safe wrapper with collection name management
#[allow(dead_code)]
pub struct ChromaVectorStore {
    client: Arc<ChromaClient>,
    collection_name: String,
}

impl ChromaVectorStore {
    pub fn new(base_url: impl Into<String>, collection_name: impl Into<String>) -> Self {
        Self {
            client: Arc::new(ChromaClient::new(base_url)),
            collection_name: collection_name.into(),
        }
    }
}

// Helper to get current collection name (in real impl, this would be stored in self)
fn config_collection_name() -> String {
    "rairos".to_string() // Placeholder - real impl would store this
}
