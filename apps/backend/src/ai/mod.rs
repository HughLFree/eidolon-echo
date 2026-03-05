//! AI abstraction layer and provider-independent chat message model.

mod openai_compat;

use anyhow::Result;
use async_trait::async_trait;

pub use openai_compat::OpenAiCompatClient;

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[async_trait]
pub trait AiClient: Send + Sync {
    async fn chat(&self, messages: &[ChatMessage]) -> Result<String>;
}
