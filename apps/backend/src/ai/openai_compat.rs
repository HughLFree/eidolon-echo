//! OpenAI-compatible chat client implementation used by configured providers.

use super::{AiClient, ChatMessage};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct OpenAiCompatClient {
    http: Client,
    base_url: String,
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ReqMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
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
    pub fn new(
        base_url: String,
        api_key: String,
        model: String,
        temperature: f32,
        max_tokens: Option<u32>,
    ) -> Self {
        Self {
            http: Client::new(),
            base_url,
            api_key,
            model,
            temperature,
            max_tokens,
        }
    }

    pub async fn chat_stream<F>(&self, messages: &[ChatMessage], mut on_delta: F) -> Result<String>
    where
        F: FnMut(&str),
    {
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
            temperature: self.temperature,
            stream: Some(true),
            max_tokens: self.max_tokens,
        };

        let mut resp = self
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

        let mut pending = String::new();
        let mut reply = String::new();

        while let Some(chunk) = resp.chunk().await? {
            let text = std::str::from_utf8(&chunk)
                .map_err(|err| anyhow!("invalid upstream utf8 chunk: {err}"))?;
            pending.push_str(text);

            while let Some(newline_idx) = pending.find('\n') {
                let mut line = pending.drain(..=newline_idx).collect::<String>();
                if line.ends_with('\n') {
                    line.pop();
                }
                if line.ends_with('\r') {
                    line.pop();
                }

                if let Some(delta) = parse_stream_delta_line(line.trim())? {
                    reply.push_str(&delta);
                    on_delta(&delta);
                }
            }
        }

        if let Some(delta) = parse_stream_delta_line(pending.trim())? {
            reply.push_str(&delta);
            on_delta(&delta);
        }

        Ok(reply)
    }
}

fn parse_stream_delta_line(line: &str) -> Result<Option<String>> {
    if line.is_empty() {
        return Ok(None);
    }

    let Some(raw_data) = line.strip_prefix("data:") else {
        return Ok(None);
    };

    let payload = raw_data.trim();
    if payload.is_empty() || payload == "[DONE]" {
        return Ok(None);
    }

    let value: Value = serde_json::from_str(payload)
        .map_err(|err| anyhow!("invalid upstream stream payload: {err}; payload: {payload}"))?;

    if let Some(error_obj) = value.get("error") {
        return Err(anyhow!("upstream error payload: {error_obj}"));
    }

    let delta = value
        .pointer("/choices/0/delta/content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if delta.is_empty() {
        return Ok(None);
    }

    Ok(Some(delta.to_string()))
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
            temperature: self.temperature,
            stream: None,
            max_tokens: self.max_tokens,
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
