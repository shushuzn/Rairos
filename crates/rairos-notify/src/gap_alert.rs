use crate::types::Result;
use crate::webhook::WebhookDispatcher;

pub struct GapAlertSender<'a> {
    dispatcher: &'a WebhookDispatcher,
    gap_type: String,
    title: String,
    novelty: f64,
    severity: String,
    supporting_papers: Vec<String>,
    source: String,
    confidence: f64,
    impact_score: f64,
}

impl<'a> GapAlertSender<'a> {
    pub fn new(
        dispatcher: &'a WebhookDispatcher,
        gap_type: &str,
        title: &str,
        novelty: f64,
        severity: &str,
    ) -> Self {
        Self {
            dispatcher,
            gap_type: gap_type.to_string(),
            title: title.to_string(),
            novelty,
            severity: severity.to_string(),
            supporting_papers: Vec::new(),
            source: "deep_research".to_string(),
            confidence: 0.0,
            impact_score: 0.0,
        }
    }

    pub fn supporting_papers(mut self, papers: Vec<String>) -> Self {
        self.supporting_papers = papers;
        self
    }

    pub fn source(mut self, source: &str) -> Self {
        self.source = source.to_string();
        self
    }

    pub fn confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn impact_score(mut self, impact_score: f64) -> Self {
        self.impact_score = impact_score;
        self
    }

    pub async fn send(self) -> Result<()> {
        self.dispatcher
            .send_gap_alert(
                &self.gap_type,
                &self.title,
                self.novelty,
                &self.severity,
                Some(&self.supporting_papers),
                Some(&self.source),
                Some(self.confidence),
                Some(self.impact_score),
            )
            .await
    }
}

pub type GapAlertBuilder = GapAlertSender<'static>;

pub fn gap_alert<'a>(
    dispatcher: &'a WebhookDispatcher,
    gap_type: &'a str,
    title: &'a str,
    novelty: f64,
    severity: &'a str,
) -> GapAlertSender<'a> {
    GapAlertSender::new(dispatcher, gap_type, title, novelty, severity)
}
