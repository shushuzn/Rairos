use crate::payloads::ContradictionEntry;
use crate::types::Result;
use crate::webhook::WebhookDispatcher;

pub struct ParadigmShiftSender<'a> {
    dispatcher: &'a WebhookDispatcher,
    alert_type: String,
    gap_type: String,
    message: String,
    severity: String,
    contradictions: Vec<ContradictionEntry>,
}

impl<'a> ParadigmShiftSender<'a> {
    pub fn new(
        dispatcher: &'a WebhookDispatcher,
        alert_type: &str,
        gap_type: &str,
        message: &str,
        severity: &str,
    ) -> Self {
        Self {
            dispatcher,
            alert_type: alert_type.to_string(),
            gap_type: gap_type.to_string(),
            message: message.to_string(),
            severity: severity.to_string(),
            contradictions: Vec::new(),
        }
    }

    pub fn contradictions(mut self, contradictions: Vec<ContradictionEntry>) -> Self {
        self.contradictions = contradictions;
        self
    }

    pub async fn send(self) -> Result<()> {
        self.dispatcher
            .send_paradigm_shift(
                &self.alert_type,
                &self.gap_type,
                &self.message,
                &self.severity,
                Some(&self.contradictions),
            )
            .await
    }
}

pub type ParadigmShiftBuilder = ParadigmShiftSender<'static>;

pub fn paradigm_shift<'a>(
    dispatcher: &'a WebhookDispatcher,
    alert_type: &'a str,
    gap_type: &'a str,
    message: &'a str,
    severity: &'a str,
) -> ParadigmShiftSender<'a> {
    ParadigmShiftSender::new(dispatcher, alert_type, gap_type, message, severity)
}
