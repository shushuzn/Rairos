//! # rairos-kg-neo4j
//!
//! Neo4j knowledge graph implementation for Rairos with Cypher query support.
//!
//! ## Overview
//!
//! This crate provides a Neo4j-backed knowledge graph that mirrors the schema
//! of `rairos-kg` (SQLite) but leverages Neo4j's graph query capabilities.
//!
//! ## Node Types
//!
//! - **Paper** — Academic papers from arXiv
//! - **Author** — Paper authors
//! - **Tag** — Research topics/tags
//! - **PNote** — Personal notes
//! - **CNote** — Citation notes
//! - **MNote** — Method notes
//! - **Figure** — Paper figures
//! - **Table** — Paper tables
//!
//! ## Edge Types
//!
//! - **cite** — Paper citations
//! - **derive** — Author → Paper relationship
//! - **same_tag** — Paper → Tag relationship
//! - **in_comparison** — M-Note comparisons
//! - **has_note** — Note associations
//! - **about_tag** — Note → Tag
//! - **has_figure** — Paper → Figure
//! - **has_table** — Paper → Table
//!
//! ## Example
//!
//! ```ignore
//! use rairos_kg_neo4j::{Neo4jKgClient, Neo4jConfig};
//!
//! let client = Neo4jKgClient::new(Neo4jConfig::default()).await?;
//! client.create_paper(&paper).await?;
//! let papers = client.get_papers_by_tag("machine-learning").await?;
//! ```

pub mod client;
pub mod schema;
pub mod cypher;
pub mod algorithms;
pub mod import;
pub mod error;

pub use client::Neo4jKgClient;
pub use schema::{KgNode, KgEdge, NodeType, EdgeType};
pub use error::KgError;
pub use cypher::CypherBuilder;
pub use algorithms::{PageRankResult, CommunityResult};
