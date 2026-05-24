//! Rairos LLM Prelude — commonly used types
//!
//! Import with: `use rairos_llm::prelude::*;`

pub use crate::{
    LlmClient, LlmCredentials, LlmError, LlmResponse,
    Message, StreamChunk, StreamResponse,
    OpenAiClient, AnthropicClient,
    QueryType, bm25_weight, mmr_lambda,
    CapsuleGene, CredibilityScorer, EvolutionEngine,
    InsightManager, CapsuleStorage, EvolutionTracker,
};
