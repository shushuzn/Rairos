//! EmbeddingMixin — Vector embedding operations for semantic paper search.

use rusqlite::{Connection, Result as SqliteResult};
use std::collections::HashMap;

/// Embedding mixin for vector similarity operations.
/// Expects the host struct to have a `_conn: Connection` field.
pub trait EmbeddingMixin {
    fn set_embedding(&self, paper_id: &str, vector: &[f32]) -> SqliteResult<bool>;
    fn get_embedding(&self, paper_id: &str) -> SqliteResult<Option<Vec<f32>>>;
    fn get_embeddings_bulk(&self, paper_ids: &[&str]) -> SqliteResult<HashMap<String, Option<Vec<f32>>>>;
    fn find_similar(&self, paper_id: &str, top_k: usize, threshold: f32) -> SqliteResult<Vec<(String, f32)>>;
    fn get_similarity(&self, paper_id1: &str, paper_id2: &str) -> SqliteResult<Option<f32>>;
}

impl<T: AsRef<Connection>> EmbeddingMixin for T {
    fn set_embedding(&self, paper_id: &str, vector: &[f32]) -> SqliteResult<bool> {
        let conn = self.as_ref();

        // Check if paper exists
        let exists = conn.query_row(
            "SELECT 1 FROM papers WHERE id = ?",
            [paper_id],
            |_| Ok(()),
        ).is_ok();

        if !exists {
            return Ok(false);
        }

        let blob = vector_to_blob(vector);
        conn.execute(
            "INSERT OR REPLACE INTO embeddings (paper_id, vector, updated_at) VALUES (?1, ?2, datetime('now'))",
            rusqlite::params![paper_id, blob],
        )?;
        Ok(true)
    }

    fn get_embedding(&self, paper_id: &str) -> SqliteResult<Option<Vec<f32>>> {
        let conn = self.as_ref();
        let row = conn.query_row(
            "SELECT vector FROM embeddings WHERE paper_id = ?",
            [paper_id],
            |row| {
                let blob: Vec<u8> = row.get(0)?;
                Ok(blob)
            },
        );

        match row {
            Ok(blob) => Ok(Some(blob_to_vector(&blob))),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn get_embeddings_bulk(&self, paper_ids: &[&str]) -> SqliteResult<HashMap<String, Option<Vec<f32>>>> {
        if paper_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let conn = self.as_ref();
        let placeholders: Vec<String> = paper_ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT paper_id, vector FROM embeddings WHERE paper_id IN ({})",
            placeholders.join(",")
        );

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(paper_ids), |row| {
            let pid: String = row.get(0)?;
            let blob: Option<Vec<u8>> = row.get(1)?;
            Ok((pid, blob))
        })?;

        let mut result: HashMap<String, Option<Vec<f32>>> = HashMap::new();
        for row in rows {
            let (pid, blob) = row?;
            result.insert(pid, blob.map(|b| blob_to_vector(&b)));
        }

        Ok(result)
    }

    fn find_similar(&self, paper_id: &str, top_k: usize, threshold: f32) -> SqliteResult<Vec<(String, f32)>> {
        let conn = self.as_ref();

        let target_vec = match self.get_embedding(paper_id)? {
            Some(v) => v,
            None => return Ok(vec![]),
        };

        let target: Vec<f32> = target_vec;
        let target_norm = norm(&target);

        let mut stmt = conn.prepare("SELECT paper_id, vector FROM embeddings WHERE paper_id != ?")?;
        let rows = stmt.query_map([paper_id], |row| {
            let pid: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            Ok((pid, blob))
        })?;

        let mut scored: Vec<(String, f32)> = Vec::new();
        for row in rows {
            let (pid, blob) = row?;
            let vec = blob_to_vector(&blob);
            let sim = cosine_similarity(&target, &vec, target_norm);
            if sim >= threshold {
                scored.push((pid, sim));
            }
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored.into_iter().take(top_k).collect())
    }

    fn get_similarity(&self, paper_id1: &str, paper_id2: &str) -> SqliteResult<Option<f32>> {
        let e1 = self.get_embedding(paper_id1)?;
        let e2 = self.get_embedding(paper_id2)?;

        match (e1, e2) {
            (Some(v1), Some(v2)) => {
                let sim = cosine_similarity(&v1, &v2, norm(&v1));
                Ok(Some(sim))
            }
            _ => Ok(None),
        }
    }
}

/// Convert f32 vector to blob bytes.
fn vector_to_blob(vector: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(vector.len() * 4);
    for &v in vector {
        blob.extend_from_slice(&v.to_le_bytes());
    }
    blob
}

/// Convert blob bytes back to f32 vector.
fn blob_to_vector(blob: &[u8]) -> Vec<f32> {
    let count = blob.len() / 4;
    let mut vec = Vec::with_capacity(count);
    for i in 0..count {
        let bytes: [u8; 4] = [blob[i*4], blob[i*4+1], blob[i*4+2], blob[i*4+3]];
        vec.push(f32::from_le_bytes(bytes));
    }
    vec
}

/// Compute L2 norm of a vector.
fn norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

/// Compute cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32], a_norm: f32) -> f32 {
    let dot = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f32>();
    let b_norm = norm(b);
    dot / (a_norm * b_norm + 1e-10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_blob_roundtrip() {
        let vec = vec![0.1, 0.2, 0.3, 0.4];
        let blob = vector_to_blob(&vec);
        let recovered = blob_to_vector(&blob);
        assert_eq!(vec.len(), recovered.len());
        for (a, b) in vec.iter().zip(recovered.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn test_cosine_similarity() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&v1, &v2, norm(&v1));
        assert!((sim - 1.0).abs() < 1e-6);
    }
}
