//! Rairos Notify — Webhook notifications for Discord, Feishu, and generic platforms.
//!
//! Ported from `notifications/dispatcher.py`. Sends rich notifications to Discord
//! (embeds) and Feishu (interactive cards) via webhooks, with a generic JSON fallback.

pub mod limits;

pub mod types;
pub use types::*;

pub mod payloads;
pub use payloads::*;

pub mod discord;
pub mod feishu;
pub mod webhook;
pub mod center;

pub mod gap_alert;
pub mod paper_ingested;
pub mod paradigm_shift;

pub use gap_alert::{GapAlertBuilder, GapAlertSender, gap_alert};
pub use paper_ingested::{PaperIngestedBuilder, PaperIngestedSender, paper_ingested};
pub use paradigm_shift::{ParadigmShiftBuilder, ParadigmShiftSender, paradigm_shift};

pub use discord::DiscordRenderer;
pub use feishu::FeishuRenderer;
pub use webhook::WebhookDispatcher;
pub use center::NotificationCenter;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discord_render_gap_alert() {
        let payload = GapAlertPayload {
            gap_type: "method_limitation".to_string(),
            title: "Attention scales poorly".to_string(),
            novelty: 0.85,
            severity: "high".to_string(),
            supporting_papers: vec!["paper1".to_string(), "paper2".to_string()],
            source: "deep_research".to_string(),
            confidence: 0.9,
            impact_score: 7.5,
        };

        let rendered = DiscordRenderer::render_gap_alert(&payload);
        assert!(rendered.get("embeds").is_some());
        let embeds = rendered["embeds"].as_array().unwrap();
        assert_eq!(embeds.len(), 1);
        let embed = &embeds[0];
        assert_eq!(embed["color"], 0xFF4444);
        assert!(embed["title"].as_str().unwrap().contains("Attention"));
    }

    #[test]
    fn test_feishu_render_gap_alert() {
        let payload = GapAlertPayload {
            gap_type: "scalability_issue".to_string(),
            title: "Scaling test".to_string(),
            novelty: 0.7,
            severity: "medium".to_string(),
            supporting_papers: vec![],
            source: "deep_research".to_string(),
            confidence: 0.0,
            impact_score: 0.0,
        };

        let rendered = FeishuRenderer::render_gap_alert(&payload);
        assert_eq!(rendered["msg_type"], "interactive");
        let header = &rendered["card"]["header"];
        assert_eq!(header["template"], "yellow");
    }

    #[test]
    fn test_generic_render() {
        let dispatcher = WebhookDispatcher::generic("http://example.com", "test");
        let payload = PaperIngestedPayload {
            title: "Test Paper".to_string(),
            arxiv_id: "1234.5678".to_string(),
            tags: vec!["ml".to_string()],
        };
        let rendered = dispatcher.render_generic("paper_ingested", &payload);
        assert_eq!(rendered["event"], "paper_ingested");
        assert_eq!(rendered["source"], "Rairos");
    }

    #[test]
    fn test_notification_type_as_str() {
        assert_eq!(NotificationType::GapAlert.as_str(), "gap_alert");
        assert_eq!(NotificationType::ParadigmShift.as_str(), "paradigm_shift");
        assert_eq!(NotificationType::PaperIngested.as_str(), "paper_ingested");
        assert_eq!(
            NotificationType::ResearchComplete.as_str(),
            "research_complete"
        );
        assert_eq!(
            NotificationType::ContradictionDetected.as_str(),
            "contradiction_detected"
        );
        assert_eq!(
            NotificationType::TopicSuggestion.as_str(),
            "topic_suggestion"
        );
    }

    #[test]
    fn test_platform_default_is_generic() {
        assert_eq!(Platform::default(), Platform::Generic);
    }

    #[test]
    fn test_discord_renderer_title_case() {
        assert_eq!(
            DiscordRenderer::title_case("method_limitation"),
            "Method Limitation"
        );
        assert_eq!(
            DiscordRenderer::title_case("scalability_issue"),
            "Scalability Issue"
        );
        assert_eq!(DiscordRenderer::title_case("rl_scaling"), "Rl Scaling");
    }

    #[test]
    fn test_discord_renderer_color_for_severity() {
        assert_eq!(
            DiscordRenderer::color_for("unknown_gap_type", "high"),
            0xFF4444
        );
        assert_eq!(
            DiscordRenderer::color_for("unknown_gap_type", "medium"),
            0xFFAA00
        );
        assert_eq!(
            DiscordRenderer::color_for("unknown_gap_type", "low"),
            0x44FF44
        );
    }

    #[test]
    fn test_discord_renderer_color_for_gap_type() {
        assert_eq!(
            DiscordRenderer::color_for("method_limitation", "unknown"),
            0xCC88FF
        );
        assert_eq!(
            DiscordRenderer::color_for("scalability_issue", "unknown"),
            0xFF8800
        );
        assert_eq!(
            DiscordRenderer::color_for("evaluation_gap", "unknown"),
            0x88CCFF
        );
        assert_eq!(
            DiscordRenderer::color_for("contradiction", "unknown"),
            0xFF4444
        );
        assert_eq!(
            DiscordRenderer::color_for("unexplored_application", "unknown"),
            0x44FFAA
        );
        assert_eq!(
            DiscordRenderer::color_for("dataset_gap", "unknown"),
            0xFFFF44
        );
    }

    #[test]
    fn test_feishu_renderer_title_case() {
        assert_eq!(
            FeishuRenderer::title_case("contradiction_cluster"),
            "Contradiction Cluster"
        );
        assert_eq!(
            FeishuRenderer::title_case("polarity_reversal"),
            "Polarity Reversal"
        );
    }

    #[test]
    fn test_feishu_renderer_severity_emoji() {
        assert_eq!(FeishuRenderer::severity_emoji("high"), "🔴");
        assert_eq!(FeishuRenderer::severity_emoji("medium"), "🟡");
        assert_eq!(FeishuRenderer::severity_emoji("low"), "🟢");
        assert_eq!(FeishuRenderer::severity_emoji("unknown"), "⚪");
    }

    #[test]
    fn test_feishu_renderer_template() {
        assert_eq!(FeishuRenderer::feishu_template("high"), "red");
        assert_eq!(FeishuRenderer::feishu_template("medium"), "yellow");
        assert_eq!(FeishuRenderer::feishu_template("low"), "green");
        assert_eq!(FeishuRenderer::feishu_template("unknown"), "grey");
    }

    #[test]
    fn test_webhook_dispatcher_factory_methods() {
        let d = WebhookDispatcher::discord("https://discord.com/webhook/123", "my-discord");
        assert_eq!(d.platform, Platform::Discord);
        assert_eq!(d.label, "my-discord");

        let f = WebhookDispatcher::feishu("https://feishu.com/webhook", "my-feishu");
        assert_eq!(f.platform, Platform::Feishu);
        assert_eq!(f.label, "my-feishu");

        let g = WebhookDispatcher::generic("https://example.com/webhook", "my-generic");
        assert_eq!(g.platform, Platform::Generic);
        assert_eq!(g.label, "my-generic");
    }

    #[test]
    fn test_webhook_dispatcher_empty_url_creates_valid() {
        let d = WebhookDispatcher::new("", Platform::Discord, "empty");
        assert!(d.webhook_url.is_empty());
        assert_eq!(d.platform, Platform::Discord);
    }

    #[test]
    fn test_gap_alert_payload_defaults() {
        let payload = GapAlertPayload {
            gap_type: "test".to_string(),
            title: "Test".to_string(),
            novelty: 0.5,
            severity: "medium".to_string(),
            supporting_papers: vec![],
            source: "deep_research".to_string(),
            confidence: 0.0,
            impact_score: 0.0,
        };
        assert_eq!(payload.source, "deep_research");
        assert!(payload.supporting_papers.is_empty());
    }

    #[test]
    fn test_contradiction_entry_serde() {
        let entry = ContradictionEntry {
            paper_a: "paper_a".to_string(),
            paper_b: "paper_b".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: ContradictionEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.paper_a, "paper_a");
        assert_eq!(parsed.paper_b, "paper_b");
    }

    #[test]
    fn test_notification_center_new() {
        let center = NotificationCenter::new();
        assert!(center.dispatchers().is_empty());
    }

    #[test]
    fn test_notification_center_add_remove() {
        let mut center = NotificationCenter::new();
        let d = WebhookDispatcher::discord("https://discord.com/webhook/123", "test");
        center.add(d.clone());
        assert_eq!(center.dispatchers().len(), 1);
        assert!(center.remove("test"));
        assert!(!center.remove("nonexistent"));
        assert!(center.dispatchers().is_empty());
    }

    #[test]
    fn test_discord_render_paradigm_shift_no_contradictions() {
        let payload = ParadigmShiftPayload {
            alert_type: "polarity_reversal".to_string(),
            gap_type: "ScalingLaws".to_string(),
            message: "Evidence shows opposite scaling behavior".to_string(),
            severity: "medium".to_string(),
            contradictions: vec![],
        };
        let rendered = DiscordRenderer::render_paradigm_shift(&payload);
        let embeds = rendered["embeds"].as_array().unwrap();
        let embed = &embeds[0];
        let fields = embed["fields"].as_array().unwrap();
        assert!(!fields.iter().any(|f| f["name"] == "Sample Contradiction"));
    }

    #[test]
    fn test_discord_render_paper_ingested_no_tags() {
        let rendered = DiscordRenderer::render_paper_ingested("Test Paper", "1234.5678", &[]);
        let embeds = rendered["embeds"].as_array().unwrap();
        let embed = &embeds[0];
        let fields = embed["fields"].as_array().unwrap();
        assert!(!fields.iter().any(|f| f["name"] == "Tags"));
    }

    #[test]
    fn test_feishu_render_paradigm_shift_medium() {
        let payload = ParadigmShiftPayload {
            alert_type: "contradiction_cluster".to_string(),
            gap_type: "Scaling".to_string(),
            message: "A contradiction was found".to_string(),
            severity: "medium".to_string(),
            contradictions: vec![],
        };
        let rendered = FeishuRenderer::render_paradigm_shift(&payload);
        assert_eq!(rendered["card"]["header"]["template"], "yellow");
    }

    #[test]
    fn test_feishu_render_paradigm_shift_high() {
        let payload = ParadigmShiftPayload {
            alert_type: "contradiction_cluster".to_string(),
            gap_type: "Scaling".to_string(),
            message: "A critical contradiction".to_string(),
            severity: "high".to_string(),
            contradictions: vec![],
        };
        let rendered = FeishuRenderer::render_paradigm_shift(&payload);
        assert_eq!(rendered["card"]["header"]["template"], "red");
    }

    #[test]
    fn test_gap_alert_payload_all_fields() {
        let payload = GapAlertPayload {
            gap_type: "method_limitation".to_string(),
            title: "Attention is all you need".to_string(),
            novelty: 0.95,
            severity: "high".to_string(),
            supporting_papers: vec![
                "paper1".to_string(),
                "paper2".to_string(),
                "paper3".to_string(),
            ],
            source: "deep_research".to_string(),
            confidence: 0.88,
            impact_score: 8.5,
        };
        assert_eq!(payload.gap_type, "method_limitation");
        assert_eq!(payload.novelty, 0.95);
        assert_eq!(payload.severity, "high");
        assert_eq!(payload.supporting_papers.len(), 3);
        assert_eq!(payload.source, "deep_research");
        assert_eq!(payload.confidence, 0.88);
        assert_eq!(payload.impact_score, 8.5);
    }

    #[test]
    fn test_notification_center_multi_dispatcher_empty_url_skipped() {
        let mut center = NotificationCenter::new();
        let d = WebhookDispatcher::new("", Platform::Discord, "empty");
        center.add(d);
        assert_eq!(center.dispatchers().len(), 1);
    }
}
