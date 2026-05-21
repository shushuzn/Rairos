//! # rairos-tools
//!
//! Material science tools for Rairos multi-agent framework.
//!
//! This crate provides a unified interface for material science tools
//! used in the SparksMatter-style multi-agent workflow.
//!
//! ## Core Trait
//!
//! - [`MaterialTool`] - Unified interface for material science tools
//!
//! ## Built-in Tools
//!
//! - [`mp::MaterialsProjectTool`] - Materials Project structure retrieval
//! - [`cgcnn::CgcnnRegressor`] - CGCNN property prediction
//! - [`mattergen::MatterGenGenerator`] - MatterGen crystal generation
//!
//! ## Example
//!
//! ```ignore
//! use rairos_tools::{MaterialTool, mp::MaterialsProjectTool};
//!
//! let tool = MaterialsProjectTool::new("your-api-key");
//! let output = tool.execute(params).await?;
//! ```

pub mod error;
pub mod tool_trait;
pub mod mp;
pub mod cgcnn;
pub mod mattergen;

pub use error::ToolError;
pub use tool_trait::{MaterialTool, ToolParams, ToolOutput};
