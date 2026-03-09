//! AI abstraction layer and provider-independent chat message model.

mod context_manager;
mod openai_compat;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use context_manager::ContextManager;
pub use openai_compat::OpenAiCompatClient;

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiRoleMode {
    #[default]
    Default,
    Roleplay,
}

#[async_trait]
pub trait AiClient: Send + Sync {
    async fn chat(&self, messages: &[ChatMessage]) -> Result<String>;
}
