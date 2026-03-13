//! HTTP handlers grouped by resource.

use axum::http::StatusCode;
use serde::Serialize;

mod chat;
mod health;
mod profiles;
mod providers;
mod runtime_ai;

pub use chat::{chat, list_messages};
pub use health::{health, openapi_yaml};
pub use profiles::{create_profile, delete_profile, get_profile, list_profiles, update_profile};
pub use providers::{
    create_ai_provider, delete_ai_provider, get_ai_provider, list_ai_providers, update_ai_provider,
};

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub deleted: bool,
}

pub(super) fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

pub(super) fn sanitize_non_empty(raw: &str, field: &str) -> Result<String, (StatusCode, String)> {
    let value = raw.trim();
    if value.is_empty() {
        return Err((StatusCode::BAD_REQUEST, format!("{field} is empty")));
    }
    Ok(value.to_string())
}

pub(super) fn sanitize_optional(raw: Option<String>) -> Option<String> {
    raw.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

pub(super) fn profile_memory_enabled(profile: Option<&crate::db::Profile>) -> bool {
    profile.map(|value| value.memory_enabled).unwrap_or(false)
}

pub(super) fn profile_context_limit(profile: Option<&crate::db::Profile>, fallback: usize) -> usize {
    let fallback = fallback.max(1).min(200);
    let Some(profile) = profile else {
        return fallback;
    };
    profile.context_limit.clamp(1, 200) as usize
}
