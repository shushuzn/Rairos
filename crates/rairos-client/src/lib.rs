use std::collections::HashMap;

pub const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
pub const MINIMAX_BASE_URL: &str = "https://api.minimax.chat/v1";
pub const ANTHROPIC_API_VERSION: &str = "2023-06-01";

pub struct LLMClient {
    pub model: String,
    pub base_url: String,
    pub api_key: String,
}

impl LLMClient {
    pub fn new(model: String, base_url: String, api_key: String) -> Self {
        Self {
            model,
            base_url,
            api_key,
        }
    }

    pub fn chat(
        &self,
        messages: Vec<HashMap<String, String>>,
        user_prompt: Option<&str>,
        system_prompt: Option<&str>,
    ) -> Result<String, String> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.chat_async(messages, user_prompt, system_prompt))
    }

    pub async fn chat_async(
        &self,
        messages: Vec<HashMap<String, String>>,
        user_prompt: Option<&str>,
        system_prompt: Option<&str>,
    ) -> Result<String, String> {
        if self.model.starts_with("claude") || self.model.contains("claude") {
            return self.call_anthropic_api(messages, user_prompt, system_prompt).await;
        }

        self.call_openai_compatible(messages, user_prompt, system_prompt).await
    }

    async fn call_openai_compatible(
        &self,
        mut messages: Vec<HashMap<String, String>>,
        user_prompt: Option<&str>,
        system_prompt: Option<&str>,
    ) -> Result<String, String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        if let Some(sys) = system_prompt {
            let mut system_msg = HashMap::new();
            system_msg.insert("role".to_string(), "system".to_string());
            system_msg.insert("content".to_string(), sys.to_string());
            messages.insert(0, system_msg);
        }

        if let Some(prompt) = user_prompt {
            let mut user_msg = HashMap::new();
            user_msg.insert("role".to_string(), "user".to_string());
            user_msg.insert("content".to_string(), prompt.to_string());
            messages.push(user_msg);
        }

        let mut payload = HashMap::new();
        payload.insert("model".to_string(), self.model.clone());
        payload.insert("temperature".to_string(), "0.2".to_string());
        payload.insert("messages".to_string(), serde_json::to_string(&messages).unwrap_or_default());

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API error: {}", response.status()));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(content) = body.get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|s| s.as_str())
        {
            Ok(content.to_string())
        } else {
            Err("Invalid response format".to_string())
        }
    }

    async fn call_anthropic_api(
        &self,
        messages: Vec<HashMap<String, String>>,
        user_prompt: Option<&str>,
        system_prompt: Option<&str>,
    ) -> Result<String, String> {
        let url = ANTHROPIC_API_URL;

        let mut anthropic_messages: Vec<HashMap<String, String>> = Vec::new();

        if let Some(sys) = system_prompt {
            let mut sys_msg = HashMap::new();
            sys_msg.insert("role".to_string(), "user".to_string());
            sys_msg.insert("content".to_string(), sys.to_string());
            anthropic_messages.push(sys_msg);
        }

        for msg in messages {
            let role = msg.get("role").cloned().unwrap_or_else(|| "user".to_string());
            if role == "system" {
                if !anthropic_messages.is_empty() {
                    anthropic_messages.insert(0, {
                        let mut m = HashMap::new();
                        m.insert("role".to_string(), "user".to_string());
                        m.insert("content".to_string(), msg.get("content").cloned().unwrap_or_default());
                        m
                    });
                } else {
                    anthropic_messages.push({
                        let mut m = HashMap::new();
                        m.insert("role".to_string(), "user".to_string());
                        m.insert("content".to_string(), msg.get("content").cloned().unwrap_or_default());
                        m
                    });
                }
            } else if role == "user" || role == "assistant" {
                anthropic_messages.push({
                    let mut m = HashMap::new();
                    m.insert("role".to_string(), role);
                    m.insert("content".to_string(), msg.get("content").cloned().unwrap_or_default());
                    m
                });
            }
        }

        if let Some(prompt) = user_prompt {
            let mut user_msg = HashMap::new();
            user_msg.insert("role".to_string(), "user".to_string());
            user_msg.insert("content".to_string(), prompt.to_string());
            anthropic_messages.push(user_msg);
        }

        let mut payload = HashMap::new();
        payload.insert("model".to_string(), self.model.clone());
        payload.insert("messages".to_string(), serde_json::to_string(&anthropic_messages).unwrap_or_default());
        payload.insert("max_tokens".to_string(), "4096".to_string());
        payload.insert("temperature".to_string(), "0.2".to_string());

        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API error: {}", response.status()));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(content_array) = body.get("content").and_then(|c| c.as_array()) {
            for block in content_array {
                if block.get("type") == Some(&serde_json::Value::String("text".to_string())) {
                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                        return Ok(text.to_string());
                    }
                }
            }
        }

        Err("No text content in Anthropic response".to_string())
    }

    pub fn generate(&self, prompt: &str, system: &str) -> String {
        let messages: Vec<HashMap<String, String>> = vec![];
        if system.is_empty() {
            self.chat(messages, Some(prompt), None).unwrap_or_default()
        } else {
            self.chat(messages, Some(prompt), Some(system)).unwrap_or_default()
        }
    }
}

pub fn get_client(model: &str, base_url: &str, api_key: &str) -> LLMClient {
    LLMClient::new(model.to_string(), base_url.to_string(), api_key.to_string())
}

pub fn is_anthropic_model(model: &str) -> bool {
    model.starts_with("claude") || model.to_lowercase().contains("claude")
}

pub fn is_ollama_model(model: &str) -> bool {
    model.starts_with("ollama/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = LLMClient::new(
            "gpt-4".to_string(),
            "https://api.openai.com/v1".to_string(),
            "test-key".to_string(),
        );
        assert_eq!(client.model, "gpt-4");
    }

    #[test]
    fn test_is_anthropic_model() {
        assert!(is_anthropic_model("claude-3-5-sonnet-latest"));
        assert!(is_anthropic_model("claude-sonnet-4"));
        assert!(!is_anthropic_model("gpt-4"));
    }

    #[test]
    fn test_is_ollama_model() {
        assert!(is_ollama_model("ollama/llama3"));
        assert!(!is_ollama_model("gpt-4"));
    }

    #[test]
    fn test_constants() {
        assert!(!ANTHROPIC_API_URL.is_empty());
        assert!(!MINIMAX_BASE_URL.is_empty());
        assert!(!ANTHROPIC_API_VERSION.is_empty());
    }

    #[test]
    fn test_get_client() {
        let client = get_client("gpt-4", "https://api.openai.com/v1", "key");
        assert_eq!(client.model, "gpt-4");
    }
}
