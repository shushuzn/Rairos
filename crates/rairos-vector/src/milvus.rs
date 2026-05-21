//! Milvus vector database client

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::client::{DistanceMetric, SearchHit, VectorStore, VectorStoreConfig};
use crate::error::VectorError;

/// Milvus API client
pub struct MilvusClient {
    base_url: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl MilvusClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_api_key(base_url, None)
    }

    pub fn with_api_key(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
            client: reqwest::Client::new(),
        }
    }

    async fn request<T: for<'de> Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T, VectorError> {
        let url = format!("{}/v1/vector{}", self.base_url, path);
        let mut request = self.client.request(method, &url);

        if let Some(ref api_key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| VectorError::ApiError(format!("Milvus request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(VectorError::ApiError(format!("Milvus API error {}: {}", status, body)));
        }

        let resp: MilvusResponse<T> = response
            .json()
            .await
            .map_err(|e| VectorError::ApiError(format!("Milvus parse error: {}", e)))?;

        match resp {
            MilvusResponse::Success(data) => Ok(data),
            MilvusResponse::Error { code, message } => {
                Err(VectorError::ApiError(format!("Milvus error {}: {}", code, message)))
            }
        }
    }

    fn metric_to_milvus(metric: DistanceMetric) -> &'static str {
        match metric {
            DistanceMetric::Cosine => "COSINE",
            DistanceMetric::Euclidean => "L2",
            DistanceMetric::DotProduct => "IP",
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum MilvusResponse<T> {
    Success(T),
    Error { code: i32, message: String },
}

#[derive(Serialize)]
struct CollectionReq {
    collection_name: String,
}

#[derive(Serialize)]
struct CreateCollectionReq {
    collection_name: String,
    dimension: usize,
    metric_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Serialize)]
struct InsertReq {
    collection_name: String,
    vectors: Vec<HashMap<String, serde_json::Value>>,
}

#[derive(Serialize)]
struct SearchReq {
    collection_name: String,
    vectors: Vec<Vec<f32>>,
    top_k: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<String>,
}

#[derive(Deserialize)]
struct SearchResp {
    results: Vec<SearchResult>,
}

#[derive(Deserialize)]
struct SearchResult {
    id: String,
    score: f32,
    #[serde(default)]
    vector: Option<Vec<f32>>,
}

#[derive(Serialize)]
struct DeleteReq {
    collection_name: String,
    #[serde(rename = "filter")]
    filter: String,
}

#[derive(Deserialize)]
struct QueryResp {
    data: Vec<serde_json::Value>,
}

#[async_trait]
impl VectorStore for MilvusClient {
    async fn initialize(&self, config: &VectorStoreConfig) -> Result<(), VectorError> {
        // Create collection if not exists
        let body = CreateCollectionReq {
            collection_name: config.collection_name.clone(),
            dimension: config.dimensions,
            metric_type: Self::metric_to_milvus(config.distance_metric).to_string(),
            description: Some("Rairos vector collection".to_string()),
        };

        // Milvus create returns 200 even if exists, so we ignore errors
        let _: serde_json::Value = self
            .request(reqwest::Method::POST, "/collections/create", Some(serde_json::to_value(body).unwrap()))
            .await
            .unwrap_or(serde_json::json!({"code": 0}));

        Ok(())
    }

    async fn upsert_one(
        &self,
        id: &str,
        embedding: &[f32],
        payload: Option<serde_json::Value>,
    ) -> Result<(), VectorError> {
        let mut vector_data: HashMap<String, serde_json::Value> = HashMap::new();
        vector_data.insert("id".to_string(), serde_json::json!(id));
        vector_data.insert("vector".to_string(), serde_json::json!(embedding));

        if let Some(p) = payload {
            if let Some(obj) = p.as_object() {
                for (k, v) in obj {
                    vector_data.insert(k.clone(), v.clone());
                }
            }
        }

        let body = InsertReq {
            collection_name: "rairos".to_string(),
            vectors: vec![vector_data],
        };

        let _: serde_json::Value = self
            .request(
                reqwest::Method::POST,
                "/entities/insert",
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

        let vectors_data: Vec<HashMap<String, serde_json::Value>> = vectors
            .into_iter()
            .map(|(id, embedding, payload)| {
                let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                data.insert("id".to_string(), serde_json::json!(id));
                data.insert("vector".to_string(), serde_json::json!(embedding));

                if let Some(p) = payload {
                    if let Some(obj) = p.as_object() {
                        for (k, v) in obj {
                            data.insert(k.clone(), v.clone());
                        }
                    }
                }
                data
            })
            .collect();

        let body = InsertReq {
            collection_name: "rairos".to_string(),
            vectors: vectors_data,
        };

        let _: serde_json::Value = self
            .request(
                reqwest::Method::POST,
                "/entities/insert",
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
        let body = SearchReq {
            collection_name: "rairos".to_string(),
            vectors: vec![query.to_vec()],
            top_k,
            filter: filter.map(|s| s.to_string()),
        };

        let resp: SearchResp = self
            .request(
                reqwest::Method::POST,
                "/entities/search",
                Some(serde_json::to_value(body).unwrap()),
            )
            .await?;

        let hits: Vec<SearchHit> = resp
            .results
            .into_iter()
            .map(|r| SearchHit {
                id: r.id,
                score: r.score,
                payload: None,
                embedding: r.vector,
            })
            .collect();

        Ok(hits)
    }

    async fn delete(&self, id: &str) -> Result<(), VectorError> {
        let body = DeleteReq {
            collection_name: "rairos".to_string(),
            filter: format!("id == '{}'", id),
        };

        let _: serde_json::Value = self
            .request(
                reqwest::Method::POST,
                "/entities/delete",
                Some(serde_json::to_value(body).unwrap()),
            )
            .await?;

        Ok(())
    }

    async fn get(&self, _id: &str) -> Result<Option<SearchHit>, VectorError> {
        // Milvus doesn't have a direct get by ID, would need query
        Ok(None)
    }

    async fn count(&self) -> Result<usize, VectorError> {
        let body = serde_json::json!({
            "collection_name": "rairos"
        });

        let resp: QueryResp = self
            .request(
                reqwest::Method::POST,
                "/entities/query",
                Some(body),
            )
            .await?;

        Ok(resp.data.len())
    }

    async fn exists(&self, _id: &str) -> Result<bool, VectorError> {
        // Would need query to check
        Ok(false)
    }

    async fn drop_collection(&self) -> Result<(), VectorError> {
        let body = serde_json::json!({
            "collection_name": "rairos"
        });

        let _: serde_json::Value = self
            .request(
                reqwest::Method::POST,
                "/collections/drop",
                Some(body),
            )
            .await?;

        Ok(())
    }
}
