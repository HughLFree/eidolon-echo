//! HTTP handlers for health check, chat, history list and OpenAPI output.

use crate::{
    ai::{AiRoleMode, ChatMessage},
    db, AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const HISTORY_PAGE_SIZE: i64 = 10;

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub session_id: Option<i64>,
    pub message: String,
    pub mode: Option<AiRoleMode>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub session_id: i64,
    pub mode: AiRoleMode,
    pub reply: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    pub limit: Option<i64>,
    pub mode: Option<AiRoleMode>,
    pub before_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct MessageListResponse {
    pub session_id: i64,
    pub messages: Vec<db::StoredMessage>,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

pub async fn openapi_yaml() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/yaml; charset=utf-8")],
        include_str!("../openapi/openapi.yaml"),
    )
}

pub async fn chat(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    let user_message = payload.message.trim();
    if user_message.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "message is empty".to_string()));
    }

    let mode = payload.mode.unwrap_or_default();
    let context_need = (state.chat_history_limit as usize).saturating_mul(2);

    if mode == AiRoleMode::Roleplay {
        if let Some(session_id) = payload.session_id {
            ensure_roleplay_cache_for_context(&state, session_id, context_need).await?;
        }
    }

    let (session_id, ai_messages): (i64, Vec<ChatMessage>) = {
        let mut manager = state
            .context_manager
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        manager.prepare_chat_messages(
            payload.session_id,
            mode,
            user_message,
            state.prompts.system_prompt.as_deref(),
            state.prompts.context_prompt.as_deref(),
            state.chat_history_limit as usize,
        )
    };

    let reply = state
        .ai_client
        .chat(&ai_messages)
        .await
        .map_err(internal_error)?;

    {
        let mut manager = state
            .context_manager
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        manager.record_exchange(session_id, mode, user_message, &reply);
        if mode == AiRoleMode::Roleplay {
            manager.reserve_roleplay_history_db_sync(session_id);
        }
    }

    Ok(Json(ChatResponse {
        session_id,
        mode,
        reply,
    }))
}

pub async fn list_messages(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<i64>,
    Query(query): Query<MessageQuery>,
) -> Result<Json<MessageListResponse>, (StatusCode, String)> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let before_id = query.before_id;
    let mode = query.mode.unwrap_or_default();

    let messages = if mode == AiRoleMode::Roleplay {
        ensure_roleplay_cache_for_history(&state, session_id, before_id, limit as usize).await?
    } else {
        let manager = state
            .context_manager
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        manager.list_cached_messages_desc(session_id, mode, limit as usize, before_id)
    };

    Ok(Json(MessageListResponse {
        session_id,
        messages,
    }))
}

async fn ensure_roleplay_cache_for_context(
    state: &Arc<AppState>,
    session_id: i64,
    needed: usize,
) -> Result<(), (StatusCode, String)> {
    loop {
        let meta = {
            let manager = state
                .context_manager
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            manager.roleplay_cache_meta(session_id)
        };

        if !meta.hydrated {
            let bootstrap_limit = HISTORY_PAGE_SIZE;
            let latest = db::list_messages(&state.pool, session_id, bootstrap_limit)
                .await
                .map_err(internal_error)?;
            let exhausted = latest.len() < bootstrap_limit as usize;
            let mut manager = state
                .context_manager
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            manager.hydrate_roleplay_latest_from_db(session_id, latest, exhausted);
            continue;
        }

        if meta.cached_len >= needed || meta.db_exhausted {
            return Ok(());
        }

        let Some(oldest_id) = meta.oldest_cached_id else {
            return Ok(());
        };
        let pull_limit = needed
            .saturating_sub(meta.cached_len)
            .max(HISTORY_PAGE_SIZE as usize)
            .min(200) as i64;
        let older = db::list_messages_before_id(&state.pool, session_id, oldest_id, pull_limit)
            .await
            .map_err(internal_error)?;
        let exhausted = older.len() < pull_limit as usize;
        let is_empty = older.is_empty();
        let mut manager = state
            .context_manager
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        manager.prepend_roleplay_older_from_db(session_id, older, exhausted);
        if is_empty {
            return Ok(());
        }
    }
}

async fn ensure_roleplay_cache_for_history(
    state: &Arc<AppState>,
    session_id: i64,
    before_id: Option<i64>,
    limit: usize,
) -> Result<Vec<db::StoredMessage>, (StatusCode, String)> {
    let effective_limit = limit.clamp(1, 200);

    loop {
        let (meta, cached) = {
            let manager = state
                .context_manager
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let meta = manager.roleplay_cache_meta(session_id);
            let cached = manager.list_cached_messages_desc(
                session_id,
                AiRoleMode::Roleplay,
                effective_limit,
                before_id,
            );
            (meta, cached)
        };

        if !meta.hydrated {
            let bootstrap_limit = HISTORY_PAGE_SIZE;
            let latest = db::list_messages(&state.pool, session_id, bootstrap_limit)
                .await
                .map_err(internal_error)?;
            let exhausted = latest.len() < bootstrap_limit as usize;
            let mut manager = state
                .context_manager
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            manager.hydrate_roleplay_latest_from_db(session_id, latest, exhausted);
            continue;
        }

        if cached.len() >= effective_limit || meta.db_exhausted {
            return Ok(cached);
        }

        let Some(oldest_id) = meta.oldest_cached_id else {
            return Ok(cached);
        };
        let pull_limit = HISTORY_PAGE_SIZE
            .max(effective_limit.saturating_sub(cached.len()) as i64)
            .min(200);
        let older = db::list_messages_before_id(&state.pool, session_id, oldest_id, pull_limit)
            .await
            .map_err(internal_error)?;
        let exhausted = older.len() < pull_limit as usize;
        let is_empty = older.is_empty();
        let mut manager = state
            .context_manager
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        manager.prepend_roleplay_older_from_db(session_id, older, exhausted);
        if is_empty {
            return Ok(cached);
        }
    }
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
