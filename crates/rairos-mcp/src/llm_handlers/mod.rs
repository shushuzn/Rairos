pub mod helpers;
pub mod generation;
pub mod gap;
pub mod citation_chain;
pub mod evolution;
pub mod research;
pub mod memory;
pub mod replication;
pub mod routing;
pub mod paper;

pub use helpers::{gene_pool_data_dir, llm_client, llm_model};
pub use generation::{
    BriefingGenerateHandler, LitReviewGenerateHandler, SlidesGenerateHandler,
    ImpactScorePaperHandler, ImpactRankHandler,
};
pub use gap::{GapDetectHandler, GapSubmitHandler, GapEvolveHandler};
pub use citation_chain::{
    CitationChainBuildHandler, CitationChainFamiliesHandler,
    CitationChainSilentHandler, CitationChainRenderHandler,
};
pub use evolution::{GenePoolDecayHandler, CrossoverHandler, GenePoolWatcherHandler};
pub use research::{
    TopicDiscoveryHandler, OrchestratorRunCycleHandler, DeepResearchRunHandler,
    ParallelResearchRunHandler, ResearchRunHandler, HypothesisGenerateHandler,
    HypothesisListHandler, RobustRankHandler, EvolutionCycleHandler,
};
pub use memory::{
    ResearchMemoryAddStanceHandler, ResearchMemoryListStancesHandler,
    ResearchMemoryCheckPaperHandler, ResearchMemoryAnomaliesHandler,
    LeaderboardHandler, ImpactLeaderboardHandler, ClaimGraphHandler,
    TagAllHandler, ReviewListHandler, ExperimentRecordHandler,
    LitReviewListHandler, ReviewSimulateHandler,
};
pub use replication::{ReplicationCheckHandler, ReplicationCompareHandler};
pub use routing::{
    TrustScorerComputeHandler, RouteQueryHandler, RoutePlanListHandler,
    RoutePlanUpdateStepHandler, RoutePlanReviseHandler,
};
pub use paper::{PaperCompareHandler, PaperAnalyzeMcpHandler};

pub async fn register_llm_handlers(server: &crate::McpServer) {
    tracing::debug!("registering 25 llm-backed MCP tool handlers");
    server.register(BriefingGenerateHandler).await;
    server.register(LitReviewGenerateHandler).await;
    server.register(SlidesGenerateHandler).await;
    server.register(GapDetectHandler).await;
    server.register(CitationChainBuildHandler).await;
    server.register(CitationChainFamiliesHandler).await;
    server.register(CitationChainSilentHandler).await;
    server.register(CitationChainRenderHandler).await;
    server.register(ImpactScorePaperHandler).await;
    server.register(ImpactRankHandler).await;
    server.register(ReplicationCheckHandler).await;
    server.register(RouteQueryHandler).await;
    server.register(TrustScorerComputeHandler).await;
    server.register(PaperCompareHandler).await;
    server.register(PaperAnalyzeMcpHandler).await;
    server.register(GapSubmitHandler).await;
    server.register(GapEvolveHandler).await;
    server.register(GenePoolDecayHandler).await;
    server.register(CrossoverHandler).await;
    server.register(ResearchMemoryAddStanceHandler).await;
    server.register(ResearchMemoryListStancesHandler).await;
    server.register(ResearchMemoryCheckPaperHandler).await;
    server.register(ResearchMemoryAnomaliesHandler).await;
    server.register(LeaderboardHandler).await;
    server.register(ImpactLeaderboardHandler).await;
    server.register(ClaimGraphHandler).await;
    server.register(TagAllHandler).await;
    server.register(ReviewListHandler).await;
    server.register(ExperimentRecordHandler).await;
    server.register(LitReviewListHandler).await;
    server.register(ReviewSimulateHandler).await;
    server.register(GenePoolWatcherHandler).await;
    server.register(ReplicationCompareHandler).await;
    server.register(RoutePlanListHandler).await;
    server.register(RoutePlanUpdateStepHandler).await;
    server.register(RoutePlanReviseHandler).await;
    server.register(ResearchRunHandler).await;
    server.register(HypothesisGenerateHandler).await;
    server.register(HypothesisListHandler).await;
    server.register(TopicDiscoveryHandler).await;
    server.register(OrchestratorRunCycleHandler).await;
    server.register(DeepResearchRunHandler).await;
    server.register(ParallelResearchRunHandler).await;
    server.register(RobustRankHandler).await;
    server.register(EvolutionCycleHandler).await;
}
