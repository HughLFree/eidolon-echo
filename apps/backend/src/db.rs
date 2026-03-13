//! SQLite access layer: pool initialization, migrations and conversation/message queries.

use crate::ai::AiRoleMode;
use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;
use sqlx::{migrate::Migrator, sqlite::SqliteRow, Row};

mod conversations;
mod messages;
mod pool;
mod profiles;
mod providers;

pub use conversations::{
    bootstrap_mode_profiles_and_conversations, resolve_mode_profile_and_conversation,
};
pub use messages::{append_message_pair, list_messages, list_messages_before_id};
pub use pool::init_pool;
pub use profiles::{create_profile, delete_profile, get_profile, list_profiles, update_profile};
pub use providers::{
    ai_provider_exists, create_ai_provider, delete_ai_provider, get_ai_provider,
    get_default_ai_provider, list_ai_providers, update_ai_provider,
};

pub(crate) static MIGRATOR: Migrator = sqlx::migrate!("./migrations");
pub(crate) const AUTO_DEFAULT_PROFILE_ID: &str = "__auto_default_profile";
pub(crate) const AUTO_ROLEPLAY_PROFILE_ID: &str = "__auto_roleplay_profile";

#[derive(Debug, Clone, Serialize)]
pub struct StoredMessage {
    pub id: i64,
    pub conversation_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiProvider {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub base_url: Option<String>,
    pub model_name: String,
    pub api_key: Option<String>,
    pub enabled: bool,
    pub is_default: bool,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct AiProviderUpsert {
    pub name: String,
    pub provider_type: String,
    pub base_url: Option<String>,
    pub model_name: String,
    pub api_key: Option<String>,
    pub enabled: bool,
    pub is_default: bool,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Profile {
    pub id: String,
    pub mode: AiRoleMode,
    pub name: String,
    pub avatar_path: Option<String>,
    pub system_prompt: String,
    pub opening_message: Option<String>,
    pub context_limit: i64,
    pub memory_enabled: bool,
    pub provider_id: Option<String>,
    pub extra_json: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct ProfileUpsert {
    pub mode: AiRoleMode,
    pub name: String,
    pub avatar_path: Option<String>,
    pub system_prompt: String,
    pub opening_message: Option<String>,
    pub context_limit: i64,
    pub memory_enabled: bool,
    pub provider_id: Option<String>,
    pub extra_json: Option<String>,
}

pub(crate) fn rows_to_messages(rows: Vec<SqliteRow>) -> Vec<StoredMessage> {
    let mut messages = Vec::with_capacity(rows.len());
    for row in rows {
        let created_at_raw: i64 = row.get("created_at");
        let dt = parse_unix_ts(created_at_raw);
        let conversation_id: i64 = row.get("conversation_id");
        messages.push(StoredMessage {
            id: row.get("id"),
            conversation_id,
            role: row.get("role"),
            content: row.get("content"),
            created_at: dt,
        });
    }
    messages
}

pub(crate) fn parse_unix_ts(raw: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(raw, 0).single().unwrap_or_else(Utc::now)
}

pub(crate) fn row_to_ai_provider(row: SqliteRow) -> Result<AiProvider> {
    Ok(AiProvider {
        id: row.get("id"),
        name: row.get("name"),
        provider_type: row.get("provider_type"),
        base_url: row.get("base_url"),
        model_name: row.get("model_name"),
        api_key: row.get("api_key"),
        enabled: i64_to_bool(row.get("enabled")),
        is_default: i64_to_bool(row.get("is_default")),
        temperature: row.get("temperature"),
        max_tokens: row.get("max_tokens"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub(crate) fn row_to_profile(row: SqliteRow) -> Result<Profile> {
    let mode_raw: String = row.get("mode");
    Ok(Profile {
        id: row.get("id"),
        mode: str_to_mode(&mode_raw)?,
        name: row.get("name"),
        avatar_path: row.get("avatar_path"),
        system_prompt: row.get("system_prompt"),
        opening_message: row.get("opening_message"),
        context_limit: row.get("context_limit"),
        memory_enabled: i64_to_bool(row.get("memory_enabled")),
        provider_id: row.get("provider_id"),
        extra_json: row.get("extra_json"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub(crate) fn mode_to_str(mode: AiRoleMode) -> &'static str {
    match mode {
        AiRoleMode::Default => "default",
        AiRoleMode::Roleplay => "roleplay",
    }
}

pub(crate) fn str_to_mode(mode: &str) -> Result<AiRoleMode> {
    match mode {
        "default" => Ok(AiRoleMode::Default),
        "roleplay" => Ok(AiRoleMode::Roleplay),
        other => Err(anyhow::anyhow!("invalid mode in db: {other}")),
    }
}

pub(crate) fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

pub(crate) fn i64_to_bool(value: i64) -> bool {
    value != 0
}
