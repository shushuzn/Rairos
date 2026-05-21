//! # rairos-pdf-advanced
//!
//! Advanced PDF parsing capabilities including GROBID integration,
//! named entity recognition (NER), and triple extraction.
//!
//! ## Overview
//!
//! This crate extends `rairos-pdf` with advanced document parsing
//! capabilities required for materials science literature analysis.
//!
//! ## Key Features
//!
//! - **GROBID Integration**: Header extraction, reference parsing, full PDF annotation
//! - **NER**: Named entity recognition for chemicals, materials, methods
//! - **Triple Extraction**: Subject-Predicate-Object extraction for knowledge graph construction
//!
//! ## Architecture
//!
//! ```text
//! PDF Document
//!    ├── GROBID Parser (header, references, structure)
//!    ├── Structure Analyzer (sections, paragraphs, tables)
//!    ├── NER Tagger (chemicals, materials, methods)
//!    └── Triple Extractor (relationships)
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use rairos_pdf_advanced::{GrobidClient, NerPipeline, TripleExtractor};
//!
//! let grobid = GrobidClient::new("http://localhost:8080");
//! let parsed = grobid.process_pdf(&pdf_bytes).await?;
//!
//! let ner = NerPipeline::new();
//! let entities = ner.extract(&parsed.full_text).await?;
//!
//! let extractor = TripleExtractor::new();
//! let triples = extractor.extract(&parsed.full_text).await?;
//! ```

pub mod grobid;
pub mod structure;
pub mod ner;
pub mod triple;
pub mod error;

pub use grobid::GrobidClient;
pub use structure::{StructuredDocument, Section, Paragraph, ParsedTable, ParsedFigure};
pub use ner::{NerPipeline, Entity, EntityType};
pub use triple::{TripleExtractor, Triple};
pub use error::PdfAdvancedError;
