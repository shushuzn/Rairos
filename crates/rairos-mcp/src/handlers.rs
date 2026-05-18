pub mod helpers;
pub mod tags;
pub mod paper;
pub mod pdf;
pub mod trends;
pub mod knowledge;
pub mod citation;
pub mod generation;
pub mod external;
pub mod paper_ops;
pub mod synthesis;
pub use synthesis::*;

pub use helpers::{data_dir, gene_pool_path, chrono_now, kg, read_jsonl, append_jsonl, write_jsonl, tags_path, home_dir, parse_arxiv_response};
pub use tags::{TagAddHandler, TagRemoveHandler, TagListHandler};
pub use paper::{PaperSearchHandler, PaperIngestHandler, PaperRecommendHandler, PaperQueryHandler, PaperChatHandler};
pub use pdf::{PdfDownloadHandler, PdfExtractTextHandler, PdfExtractStructuredHandler};
pub use trends::{TrendsDetectTrendingHandler, TrendsPredictNextHandler, TrendsTopPredictionsHandler, TrendsCompareTagsHandler};
pub use knowledge::{CitationGraphHandler, KgPaperSubgraphHandler, KgTagGraphHandler, KgFullGraphHandler, KgQueryHandler};
pub use citation::{CiteFetchHandler, PaperSearchMultiHandler, PaperLookupDoiHandler, PaperCitationsHandler, PaperVerifyCitationsHandler, CitationStyle, format_citation_apa, format_citation_nature, format_citation_vancouver};
pub use generation::{PaperVisualizeTrendsHandler, PaperVisualizeRadarHandler, PaperCriticalAnalysisHandler, PaperGenerateReviewPdfHandler, HypothesisReportHandler, PaperGenerateSchematicHandler};
pub use external::{PaperScienceDiscoveryHandler, PaperDatabaseLookupHandler, PaperPeerReviewHandler, PaperFormatCitationHandler, PaperLiteratureReviewHandler, ChartQueryHandler};
pub use paper_ops::{PaperParseFullHandler, ReplicationCheckSimpleHandler, GitHubRepoMetadataHandler, HuggingFaceDatasetHandler, PdfExtractAdvancedHandler};
pub use synthesis::{PaperGenerateHandler, WhatIfOracleHandler, PaperDocxHandler, PaperSlidesHandler, PaperGrantProposalHandler, StatisticalAnalysisHandler};

pub async fn register_all(server: &crate::McpServer) {
    server.register(PaperSearchHandler).await;
    server.register(PaperIngestHandler).await;
    server.register(PaperParseFullHandler).await;
    server.register(ReplicationCheckSimpleHandler).await;
    server.register(GitHubRepoMetadataHandler).await;
    server.register(HuggingFaceDatasetHandler).await;
    server.register(PdfExtractAdvancedHandler).await;
    server.register(PaperQueryHandler).await;
    server.register(PaperChatHandler).await;
    server.register(TagAddHandler).await;
    server.register(TagRemoveHandler).await;
    server.register(TagListHandler).await;
    server.register(TrendsDetectTrendingHandler).await;
    server.register(PaperRecommendHandler).await;
    server.register(CitationGraphHandler).await;
    server.register(KgPaperSubgraphHandler).await;
    server.register(KgTagGraphHandler).await;
    server.register(KgFullGraphHandler).await;
    server.register(KgQueryHandler).await;
    server.register(PdfDownloadHandler).await;
    server.register(PdfExtractTextHandler).await;
    server.register(PdfExtractStructuredHandler).await;
    server.register(TrendsPredictNextHandler).await;
    server.register(TrendsTopPredictionsHandler).await;
    server.register(TrendsCompareTagsHandler).await;
    server.register(CiteFetchHandler).await;
    server.register(PaperSearchMultiHandler).await;
    server.register(PaperLookupDoiHandler).await;
    server.register(PaperCitationsHandler).await;
    server.register(PaperVerifyCitationsHandler).await;
    server.register(PaperVisualizeTrendsHandler).await;
    server.register(PaperVisualizeRadarHandler).await;
    server.register(PaperCriticalAnalysisHandler).await;
    server.register(PaperGenerateReviewPdfHandler).await;
    server.register(HypothesisReportHandler).await;
    server.register(PaperGenerateSchematicHandler).await;
    server.register(PaperScienceDiscoveryHandler).await;
    server.register(PaperDatabaseLookupHandler).await;
    server.register(PaperPeerReviewHandler).await;
    server.register(PaperFormatCitationHandler).await;
    server.register(PaperLiteratureReviewHandler).await;
    server.register(PaperGenerateHandler).await;
    server.register(WhatIfOracleHandler).await;
    server.register(PaperDocxHandler).await;
    server.register(PaperSlidesHandler).await;
    server.register(PaperGrantProposalHandler).await;
    server.register(StatisticalAnalysisHandler).await;
    server.register(ChartQueryHandler).await;
    crate::llm_handlers::register_llm_handlers(server).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ToolHandler;

    #[test]
    fn test_tag_add_list_remove_cycle() {
        let path = std::env::temp_dir().join("test_tags.jsonl");
        let _ = std::fs::remove_file(&path);

        let entry = serde_json::json!({"arxiv_id": "2401.00001", "tag": "transformer"});
        let p = path.clone();
        append_jsonl(&p, &entry).unwrap();
        let entries = read_jsonl(&p);
        assert_eq!(entries.len(), 1);

        let entry2 = serde_json::json!({"arxiv_id": "2401.00002", "tag": "gnn"});
        append_jsonl(&p, &entry2).unwrap();
        let entries = read_jsonl(&p);
        assert_eq!(entries.len(), 2);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_schema_definitions() {
        assert!(PaperSearchHandler.name() == "paper_search");
        assert!(TagAddHandler.name() == "tag_add");
        assert!(TagRemoveHandler.name() == "tag_remove");
        assert!(TagListHandler.name() == "tag_list");
        assert!(TrendsDetectTrendingHandler.name() == "trends_detect_trending");
        assert!(PaperRecommendHandler.name() == "paper_recommend");
        assert!(CitationGraphHandler.name() == "citation_graph");
        assert!(PaperQueryHandler.name() == "paper_query");
        assert!(PaperIngestHandler.name() == "paper_ingest");
        assert!(PaperChatHandler.name() == "paper_chat");
        assert!(KgPaperSubgraphHandler.name() == "kg_paper_subgraph");
        assert!(KgTagGraphHandler.name() == "kg_tag_graph");
        assert!(KgFullGraphHandler.name() == "kg_full_graph");
        assert!(KgQueryHandler.name() == "kg_query");
        assert!(PdfDownloadHandler.name() == "pdf_download");
        assert!(PdfExtractTextHandler.name() == "pdf_extract_text");
        assert!(PdfExtractStructuredHandler.name() == "pdf_extract_structured");
        assert!(TrendsPredictNextHandler.name() == "trends_predict_next");
        assert!(TrendsTopPredictionsHandler.name() == "trends_top_predictions");
        assert!(TrendsCompareTagsHandler.name() == "trends_compare_tags");
        assert!(CiteFetchHandler.name() == "cite_fetch");
    }

    #[test]
    fn test_parse_arxiv_response() {
        let xml = r#"<?xml version="1.0"?><feed>
<entry><id>http://arxiv.org/abs/2401.12345</id><published>2024-01-01</published>
<title>Test Title</title><summary>Test abstract</summary>
<author><name>John Doe</name></author><category term="cs.LG"/>
</entry></feed>"#;
        let papers = parse_arxiv_response(xml);
        assert_eq!(papers.len(), 1);
        assert_eq!(papers[0]["arxiv_id"], "2401.12345");
    }

    #[test]
    fn test_pdf_handlers_schema_requires_arxiv_id() {
        let req = |h: &dyn crate::protocol::ToolHandler| h.input_schema().required.unwrap_or_default();
        assert!(req(&PdfDownloadHandler).contains(&"arxiv_id".into()));
        assert!(req(&PdfExtractTextHandler).contains(&"arxiv_id".into()));
        assert!(req(&PdfExtractStructuredHandler).contains(&"arxiv_id".into()));
    }

    #[test]
    fn test_pdf_download_error_missing_arxiv_id() {
        let result = futures::executor::block_on(PdfDownloadHandler.call(serde_json::json!({})));
        assert_eq!(result, Err("Missing arxiv_id".to_string()));
    }

    #[test]
    fn test_pdf_extract_text_error_missing_arxiv_id() {
        let result = futures::executor::block_on(PdfExtractTextHandler.call(serde_json::json!({})));
        assert_eq!(result, Err("Missing arxiv_id".to_string()));
    }

    #[test]
    fn test_pdf_extract_structured_error_missing_arxiv_id() {
        let result = futures::executor::block_on(PdfExtractStructuredHandler.call(serde_json::json!({})));
        assert_eq!(result, Err("Missing arxiv_id".to_string()));
    }

    #[test]
    fn test_trends_predict_next_schema_requires_tag() {
        let req = TrendsPredictNextHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"tag".into()));
    }

    #[test]
    fn test_trends_predict_next_error_missing_tag() {
        let result = futures::executor::block_on(TrendsPredictNextHandler.call(serde_json::json!({})));
        assert_eq!(result, Err("Missing tag".to_string()));
    }

    #[test]
    fn test_trends_top_predictions_no_required() {
        let schema = TrendsTopPredictionsHandler.input_schema();
        assert!(schema.required.is_none() || schema.required.as_ref().unwrap().is_empty());
    }

    #[test]
    fn test_trends_compare_tags_schema_requires_both() {
        let req = TrendsCompareTagsHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"tag_a".into()));
        assert!(req.contains(&"tag_b".into()));
    }

    #[test]
    fn test_trends_compare_tags_error_missing_tag_a() {
        let result = futures::executor::block_on(TrendsCompareTagsHandler.call(serde_json::json!({"tag_b": "test"})));
        assert_eq!(result, Err("Missing tag_a".to_string()));
    }

    #[test]
    fn test_cite_fetch_schema_requires_paper_id() {
        let req = CiteFetchHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"paper_id".into()));
    }

    #[test]
    fn test_cite_fetch_error_missing_paper_id() {
        let result = futures::executor::block_on(CiteFetchHandler.call(serde_json::json!({})));
        assert_eq!(result, Err("Missing paper_id".to_string()));
    }

    #[test]
    fn test_trends_compare_tags_error_missing_tag_b() {
        let result = futures::executor::block_on(TrendsCompareTagsHandler.call(serde_json::json!({"tag_a": "test"})));
        assert_eq!(result, Err("Missing tag_b".to_string()));
    }

    #[test]
    fn test_paper_verify_citations_schema_requires_dois() {
        let req = PaperVerifyCitationsHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"dois".into()));
    }

    #[test]
    fn test_paper_verify_citations_error_missing_dois() {
        let result = futures::executor::block_on(PaperVerifyCitationsHandler.call(serde_json::json!({})));
        assert_eq!(result, Err("Missing required parameter: dois".to_string()));
    }

    #[test]
    fn test_paper_verify_citations_error_invalid_style() {
        let result = futures::executor::block_on(PaperVerifyCitationsHandler.call(serde_json::json!({"dois": "10.1234/test", "style": "invalid"})));
        assert!(result.is_err() && result.unwrap_err().contains("Invalid style"));
    }

    #[test]
    fn test_paper_verify_citations_error_no_dois() {
        let result = futures::executor::block_on(PaperVerifyCitationsHandler.call(serde_json::json!({"dois": ""})));
        assert_eq!(result, Err("No DOIs provided".to_string()));
    }

    #[test]
    fn test_paper_visualize_trends_schema_requires_trends_json() {
        let req = PaperVisualizeTrendsHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"trends_json".into()));
    }

    #[test]
    fn test_paper_visualize_trends_error_missing_data() {
        let result = futures::executor::block_on(PaperVisualizeTrendsHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_visualize_radar_schema_requires_scores_json() {
        let req = PaperVisualizeRadarHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"scores_json".into()));
    }

    #[test]
    fn test_paper_visualize_radar_error_missing_data() {
        let result = futures::executor::block_on(PaperVisualizeRadarHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_critical_analysis_schema_requires_fields() {
        let req = PaperCriticalAnalysisHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"paper_id".into()));
        assert!(req.contains(&"title".into()));
        assert!(req.contains(&"abstract".into()));
    }

    #[test]
    fn test_paper_critical_analysis_error_missing_fields() {
        let result = futures::executor::block_on(PaperCriticalAnalysisHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_generate_review_pdf_schema_requires_review_json() {
        let req = PaperGenerateReviewPdfHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"review_json".into()));
    }

    #[test]
    fn test_paper_generate_review_pdf_error_missing_data() {
        let result = futures::executor::block_on(PaperGenerateReviewPdfHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_hypothesis_report_schema_requires_fields() {
        let req = HypothesisReportHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"topic".into()));
        assert!(req.contains(&"hypotheses_json".into()));
    }

    #[test]
    fn test_hypothesis_report_error_missing_fields() {
        let result = futures::executor::block_on(HypothesisReportHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_generate_schematic_schema_requires_fields() {
        let req = PaperGenerateSchematicHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"diagram_type".into()));
        assert!(req.contains(&"diagram_json".into()));
    }

    #[test]
    fn test_paper_generate_schematic_error_missing_fields() {
        let result = futures::executor::block_on(PaperGenerateSchematicHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_science_discovery_schema_requires_query() {
        let req = PaperScienceDiscoveryHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"query".into()));
    }

    #[test]
    fn test_paper_science_discovery_error_missing_query() {
        let result = futures::executor::block_on(PaperScienceDiscoveryHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_database_lookup_schema() {
        let req = PaperDatabaseLookupHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"query_type".into()));
        assert!(req.contains(&"term".into()));
    }

    #[test]
    fn test_paper_database_lookup_error_missing_fields() {
        let result = futures::executor::block_on(PaperDatabaseLookupHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_peer_review_schema() {
        let req = PaperPeerReviewHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"paper_id".into()));
        assert!(req.contains(&"title".into()));
    }

    #[test]
    fn test_paper_peer_review_error_missing_fields() {
        let result = futures::executor::block_on(PaperPeerReviewHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_peer_review_minimal_input() {
        let result = futures::executor::block_on(PaperPeerReviewHandler.call(serde_json::json!({
            "paper_id": "test123",
            "title": "Test Paper",
            "abstract_text": "This is a test abstract about methods and results.",
            "sections": "Introduction: background. Methods: experiments were conducted with statistical tests. Results: findings show significant effects. Discussion: limitations acknowledged."
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["overall_score"].as_f64().is_some());
        assert!(!output["recommendation"].as_str().unwrap().is_empty());
        assert!(output["major_issues"].as_array().is_some());
        assert!(output["minor_issues"].as_array().is_some());
    }

    #[test]
    fn test_paper_peer_review_consort_checklist() {
        let result = futures::executor::block_on(PaperPeerReviewHandler.call(serde_json::json!({
            "paper_id": "test456",
            "title": "Clinical Trial Paper",
            "checklist_type": "CONSORT"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["compliance"]["consort_checklist"].is_object());
    }

    #[test]
    fn test_paper_format_citation_schema() {
        let req = PaperFormatCitationHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"identifier".into()));
    }

    #[test]
    fn test_paper_format_citation_error_missing_fields() {
        let result = futures::executor::block_on(PaperFormatCitationHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_literature_review_schema() {
        let req = PaperLiteratureReviewHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"topic".into()));
    }

    #[test]
    fn test_paper_literature_review_error_missing_fields() {
        let result = futures::executor::block_on(PaperLiteratureReviewHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_what_if_oracle_schema_requires_question() {
        let req = WhatIfOracleHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"question".into()));
    }

    #[test]
    fn test_what_if_oracle_error_missing_question() {
        let result = futures::executor::block_on(WhatIfOracleHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_what_if_oracle_returns_six_branches() {
        let result = futures::executor::block_on(WhatIfOracleHandler.call(serde_json::json!({
            "question": "What if CRISPR therapy cures sickle cell disease?"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["branches"].as_array().is_some());
        assert_eq!(output["branches"].as_array().unwrap().len(), 6);
        assert!(!output["synthesis"].as_str().unwrap().is_empty());
    }

    #[test]
    fn test_what_if_oracle_quick_mode_three_branches() {
        let result = futures::executor::block_on(WhatIfOracleHandler.call(serde_json::json!({
            "question": "What if AI automates scientific discovery?",
            "mode": "quick"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["branches"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_paper_docx_export_schema() {
        let req = PaperDocxHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"markdown".into()));
        assert!(req.contains(&"title".into()));
    }

    #[test]
    fn test_paper_docx_export_error_missing_fields() {
        let result = futures::executor::block_on(PaperDocxHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_docx_export_generates_docx() {
        let result = futures::executor::block_on(PaperDocxHandler.call(serde_json::json!({
            "markdown": "# Test\n\nThis is a test paragraph.\n\n## Section\n\n- Item 1\n- Item 2",
            "title": "Test Paper",
            "filename": "test_export"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["format"], "docx");
        assert!(output["size_bytes"].as_u64().unwrap() > 0);
        assert!(output["path"].as_str().unwrap().contains("test_export.docx"));
    }

    #[test]
    fn test_paper_slides_generate_schema() {
        let req = PaperSlidesHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"topic".into()));
    }

    #[test]
    fn test_paper_slides_generate_error_missing_topic() {
        let result = futures::executor::block_on(PaperSlidesHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_slides_generate_returns_markdown() {
        let result = futures::executor::block_on(PaperSlidesHandler.call(serde_json::json!({
            "topic": "CRISPR gene therapy for sickle cell disease"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["markdown"].as_str().unwrap().contains("# Slide Deck:"));
        assert!(output["slide_count"].as_u64().unwrap() > 0);
        assert_eq!(output["talk_type"], "conference");
    }

    #[test]
    fn test_paper_slides_generate_seminar_talk() {
        let result = futures::executor::block_on(PaperSlidesHandler.call(serde_json::json!({
            "topic": "Machine learning in drug discovery",
            "talk_type": "seminar",
            "include_notes": "true"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["markdown"].as_str().unwrap().contains("Speaker Notes"));
        assert!(output["slide_count"].as_u64().unwrap() > 10);
    }

    #[test]
    fn test_paper_slides_generate_focus_areas() {
        let result = futures::executor::block_on(PaperSlidesHandler.call(serde_json::json!({
            "topic": "Climate change impact on agriculture",
            "focus_slides": "introduction,results,conclusion"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["focus_areas"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_paper_grant_proposal_schema() {
        let req = PaperGrantProposalHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"topic".into()));
        assert!(req.contains(&"agency".into()));
    }

    #[test]
    fn test_paper_grant_proposal_error_missing_fields() {
        let result = futures::executor::block_on(PaperGrantProposalHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_grant_proposal_nsf_format() {
        let result = futures::executor::block_on(PaperGrantProposalHandler.call(serde_json::json!({
            "topic": "AI for drug discovery",
            "agency": "NSF"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["agency"], "NSF");
        assert!(output["markdown"].as_str().unwrap().contains("# Grant Proposal:"));
        assert!(output["markdown"].as_str().unwrap().contains("Project Summary"));
        assert!(output["section_count"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_paper_grant_proposal_nih_format() {
        let result = futures::executor::block_on(PaperGrantProposalHandler.call(serde_json::json!({
            "topic": "CRISPR gene therapy",
            "agency": "NIH",
            "pi_name": "Dr. Jane Smith",
            "institution": "MIT",
            "funding_amount": "$2,000,000 for 5 years"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["agency"], "NIH");
        assert!(output["markdown"].as_str().unwrap().contains("Specific Aims"));
        assert!(output["markdown"].as_str().unwrap().contains("Dr. Jane Smith"));
    }

    #[test]
    fn test_paper_grant_proposal_darpa_format() {
        let result = futures::executor::block_on(PaperGrantProposalHandler.call(serde_json::json!({
            "topic": "Quantum computing applications",
            "agency": "DARPA"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["agency"], "DARPA");
        assert!(output["markdown"].as_str().unwrap().contains("Executive Summary"));
        assert!(output["markdown"].as_str().unwrap().contains("Technical Plan"));
    }

    #[test]
    fn test_statistical_analysis_guide_schema() {
        let req = StatisticalAnalysisHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"research_question".into()));
        assert!(req.contains(&"data_type".into()));
        assert!(req.contains(&"groups".into()));
    }

    #[test]
    fn test_statistical_analysis_guide_error_missing_fields() {
        let result = futures::executor::block_on(StatisticalAnalysisHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_statistical_analysis_guide_ttest() {
        let result = futures::executor::block_on(StatisticalAnalysisHandler.call(serde_json::json!({
            "research_question": "Does treatment A reduce blood pressure compared to placebo?",
            "data_type": "continuous",
            "groups": "two_independent"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["markdown"].as_str().unwrap().contains("t-test"));
        assert!(output["recommended_tests"].as_array().unwrap().len() > 0);
    }

    #[test]
    fn test_statistical_analysis_guide_correlation() {
        let result = futures::executor::block_on(StatisticalAnalysisHandler.call(serde_json::json!({
            "research_question": "Is there a relationship between exercise and anxiety?",
            "data_type": "continuous",
            "groups": "correlation"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["markdown"].as_str().unwrap().contains("Pearson"));
        assert!(output["markdown"].as_str().unwrap().contains("Spearman"));
    }

    #[test]
    fn test_statistical_analysis_guide_anova() {
        let result = futures::executor::block_on(StatisticalAnalysisHandler.call(serde_json::json!({
            "research_question": "Do three different diets have different weight loss effects?",
            "data_type": "continuous",
            "groups": "three_plus_independent",
            "hypothesis": "difference"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["markdown"].as_str().unwrap().contains("ANOVA"));
        assert!(output["markdown"].as_str().unwrap().contains("Kruskal-Wallis"));
    }
}
