//! # rairos-graphrag
//!
//! GraphRAG implementation combining vector retrieval with knowledge graph reasoning.
//!
//! ## Overview
//!
//! This crate provides a Retrieval-Augmented Generation pipeline that leverages
//! both vector similarity search and knowledge graph structure for improved
//! question answering over academic literature.
//!
//! ## Key Features
//!
//! - **Hybrid Retrieval**: Combines vector similarity with graph structure
//! - **Community Detection**: Groups related entities using graph algorithms
//! - **Multi-hop Reasoning**: Finds paths between entities through the knowledge graph
//! - **LLM Generation**: Produces grounded answers using retrieved context
//!
//! ## Architecture
//!
//! ```text
//! Question
//!    ├── Vector Search (rairos-vector)
//!    │   └── Top-K similar documents
//!    ├── Knowledge Graph Query (rairos-kg-neo4j)
//!    │   ├── Related entities
//!    │   ├── Citation paths
//!    │   └── Community context
//!    └── LLM Generation (rairos-llm)
//!        └── Grounded answer
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use rairos_graphrag::{GraphRagPipeline, GraphRagConfig};
//!
//! let config = GraphRagConfig::default();
//! let pipeline = GraphRagPipeline::new(embedder, kg_client, llm, config);
//!
//! let answer = pipeline.query("How do GNNs help materials discovery?").await?;
//! ```

pub mod pipeline;
pub mod retrieval;
pub mod community;
pub mod reasoning;
pub mod error;

pub use pipeline::GraphRagPipeline;
pub use retrieval::HybridRetriever;
pub use community::CommunitySummarizer;
pub use reasoning::PathFinder;
pub use error::GraphRagError;
pub use pipeline::{GraphRagConfig, GraphRagAnswer, Source};
