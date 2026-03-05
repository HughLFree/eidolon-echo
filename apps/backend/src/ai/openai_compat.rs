//! OpenAI-compatible chat client implementation used by configured providers.

use super::{AiClient, ChatMessage};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct OpenAiCompatClient {
    http: Client,
    base_url: String,
    api_key: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ReqMessage>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct ReqMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: RespMessage,
}

#[derive(Debug, Deserialize)]
struct RespMessage {
    content: String,
}

impl OpenAiCompatClient {
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        Self {
            http: Client::new(),
            base_url,
            api_key,
            model,
        }
    }
}

#[async_trait]
impl AiClient for OpenAiCompatClient {
    async fn chat(&self, messages: &[ChatMessage]) -> Result<String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = ChatCompletionRequest {
            model: self.model.clone(),
            messages: messages
                .iter()
                .map(|m| ReqMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                })
                .collect(),
            temperature: 0.7,
        };

        let resp = self
            .http
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp
                .text()
                .await
                .unwrap_or_else(|_| "<no body>".to_string());
            return Err(anyhow!("upstream error: {status}, body: {text}"));
        }

        let parsed: ChatCompletionResponse = resp.json().await?;
        let content = parsed
            .choices
            .first()
            .ok_or_else(|| anyhow!("upstream response contains no choices"))?
            .message
            .content
            .clone();

        Ok(content)
    }
}
