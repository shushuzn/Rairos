//! # rairos-vector
//!
//! Vector database and embedding infrastructure for Rairos RAG pipelines.
//!
//! ## Core Traits
//!
//! - [`Embedder`] - Generate embeddings from text
//! - [`VectorStore`] - Store and search vectors
//! - [`RagPipeline`] - Retrieval-Augmented Generation pipeline
//!
//! ## Providers
//!
//! - [`OpenAiEmbedder`] - OpenAI Embeddings API (text-embedding-3-small, text-embedding-3-large, ada-002)
//! - [`BgemEmbedder`] - BGE-M3 API (via MiniMax)
//! - [`ChromaClient`] - Chroma vector database
//! - [`MilvusClient`] - Milvus vector database
//! - [`FaissStore`] - FAISS local index
//!
//! ## Example
//!
//! ```ignore
//! use rairos_vector::{RagPipeline, OpenAiEmbedder, ChromaClient};
//!
//! let embedder = OpenAiEmbedder::new("sk-...");
//! let store = ChromaClient::new("http://localhost:8000");
//! let llm = /* rairos-llm LlmClient */;
//!
//! let rag = RagPipeline::new(embedder, store, llm);
//! let answer = rag.query("What are the key findings in the paper?").await?;
//! ```

pub mod embedding;
pub mod graphrag;
pub mod client;
pub mod chroma;
pub mod milvus;
pub mod faiss;
pub mod rag;
pub mod error;

pub use embedding::{Embedder, BgemEmbedder, OpenAiEmbedder, EmbeddingModel};
pub use client::{VectorStore, SearchHit, VectorStoreConfig};
pub use rag::RagPipeline;
pub use rag::{InlineCitationRag, InlineCitationConfig, CitationAnswer, CitedSegment, CitedClaim};
pub use error::VectorError;
