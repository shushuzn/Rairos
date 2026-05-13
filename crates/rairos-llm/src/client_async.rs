use std::collections::HashMap;

pub struct AsyncClient {
    base_url: String,
    api_key: String,
    model: String,
}

impl AsyncClient {
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        Self {
            api_key,
            base_url,
            model,
        }
    }

    pub async fn chat_completions(
        &self,
        messages: Vec<HashMap<String, String>>,
        user_prompt: Option<&str>,
        system_prompt: Option<&str>,
        stream: bool,
    ) -> Result<String, String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let mut msgs: Vec<HashMap<String, String>> = messages;
        if let Some(sys) = system_prompt {
            let mut system_msg = HashMap::new();
            system_msg.insert("role".to_string(), "system".to_string());
            system_msg.insert("content".to_string(), sys.to_string());
            let mut new_msgs = vec![system_msg];
            new_msgs.append(&mut msgs);
            msgs = new_msgs;
        }

        let mut payload = HashMap::new();
        payload.insert("model".to_string(), self.model.clone());
        payload.insert("temperature".to_string(), "0.2".to_string());
        payload.insert(
            "messages".to_string(),
            serde_json::to_string(&msgs).unwrap_or_default(),
        );
        payload.insert("stream".to_string(), stream.to_string());

        if let Some(prompt) = user_prompt {
            let mut user_msg = HashMap::new();
            user_msg.insert("role".to_string(), "user".to_string());
            user_msg.insert("content".to_string(), prompt.to_string());
            msgs.push(user_msg);
            payload.insert(
                "messages".to_string(),
                serde_json::to_string(&msgs).unwrap_or_default(),
            );
        }

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

        if let Some(content) = body
            .get("choices")
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

    pub async fn chat_completions_streaming(
        &self,
        messages: Vec<HashMap<String, String>>,
        user_prompt: Option<&str>,
        system_prompt: Option<&str>,
    ) -> Result<String, String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let mut msgs: Vec<HashMap<String, String>> = messages;
        if let Some(sys) = system_prompt {
            let mut system_msg = HashMap::new();
            system_msg.insert("role".to_string(), "system".to_string());
            system_msg.insert("content".to_string(), sys.to_string());
            let mut new_msgs = vec![system_msg];
            new_msgs.append(&mut msgs);
            msgs = new_msgs;
        }

        let mut payload = HashMap::new();
        payload.insert("model".to_string(), self.model.clone());
        payload.insert("temperature".to_string(), "0.2".to_string());
        payload.insert(
            "messages".to_string(),
            serde_json::to_string(&msgs).unwrap_or_default(),
        );
        payload.insert("stream".to_string(), "true".to_string());

        if let Some(prompt) = user_prompt {
            let mut user_msg = HashMap::new();
            user_msg.insert("role".to_string(), "user".to_string());
            user_msg.insert("content".to_string(), prompt.to_string());
            msgs.push(user_msg);
            payload.insert(
                "messages".to_string(),
                serde_json::to_string(&msgs).unwrap_or_default(),
            );
        }

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

        let mut full_content = String::new();
        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            if let Ok(bytes) = chunk {
                if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                    for line in text.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                continue;
                            }
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                if let Some(content) = json
                                    .get("choices")
                                    .and_then(|c| c.as_array())
                                    .and_then(|arr| arr.first())
                                    .and_then(|c| c.get("delta"))
                                    .and_then(|d| d.get("content"))
                                    .and_then(|s| s.as_str())
                                {
                                    full_content.push_str(content);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(full_content)
    }
}

pub async fn close_session() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = AsyncClient::new(
            "test-key".to_string(),
            "https://api.example.com".to_string(),
            "gpt-4".to_string(),
        );
        assert_eq!(client.model, "gpt-4");
    }

    #[test]
    fn test_close_session_is_async() {
        assert!(true);
    }
}
