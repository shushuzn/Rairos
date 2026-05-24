use crate::notify::types::Result;
use crate::notify::webhook::WebhookDispatcher;

pub struct PaperIngestedSender<'a> {
    dispatcher: &'a WebhookDispatcher,
    title: String,
    arxiv_id: String,
    tags: Vec<String>,
}

impl<'a> PaperIngestedSender<'a> {
    pub fn new(dispatcher: &'a WebhookDispatcher, title: &str, arxiv_id: &str) -> Self {
        Self {
            dispatcher,
            title: title.to_string(),
            arxiv_id: arxiv_id.to_string(),
            tags: Vec::new(),
        }
    }

    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub async fn send(self) -> Result<()> {
        self.dispatcher
            .send_paper_ingested(&self.title, &self.arxiv_id, Some(&self.tags))
            .await
    }
}

pub type PaperIngestedBuilder = PaperIngestedSender<'static>;

pub fn paper_ingested<'a>(
    dispatcher: &'a WebhookDispatcher,
    title: &'a str,
    arxiv_id: &'a str,
) -> PaperIngestedSender<'a> {
    PaperIngestedSender::new(dispatcher, title, arxiv_id)
}
