use super::{internal_error, profile_context_limit, profile_memory_enabled};
use crate::{
    ai::{AiRoleMode, ChatMessage},
    db, AppState,
};
use axum::{
    body::{Body, Bytes},
    extract::{Query, State},
    http::{
        header::{CACHE_CONTROL, CONTENT_TYPE},
        HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, sync::Arc};
use tokio_stream::wrappers::UnboundedReceiverStream;

const HISTORY_PAGE_SIZE: i64 = 10;

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub mode: Option<AiRoleMode>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub conversation_id: i64,
    pub mode: AiRoleMode,
    pub reply: String,
}

#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    pub limit: Option<i64>,
    pub mode: Option<AiRoleMode>,
    pub before_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct MessageListResponse {
    pub conversation_id: i64,
    pub messages: Vec<db::StoredMessage>,
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
    let (mode_profile, conversation_id) = db::resolve_mode_profile_and_conversation(&state.pool, mode)
        .await
        .map_err(internal_error)?;

    let memory_enabled = profile_memory_enabled(Some(&mode_profile));
    let context_limit = profile_context_limit(Some(&mode_profile), state.chat_history_limit as usize);

    if memory_enabled {
        ensure_memory_cache_for_context(&state, conversation_id, context_limit).await?;
    }

    let profile_prompt = {
        let prompt = mode_profile.system_prompt.trim();
        if prompt.is_empty() {
            None
        } else {
            Some(prompt.to_string())
        }
    };
    let system_prompt = profile_prompt
        .as_deref()
        .or(state.prompts.system_prompt.as_deref());

    let ai_messages: Vec<ChatMessage> = {
        let mut manager = state
            .context_manager
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        manager.prepare_chat_messages(
            conversation_id,
            user_message,
            system_prompt,
            state.prompts.context_prompt.as_deref(),
            context_limit,
        )
    };

    let runtime_ai_client = super::runtime_ai::resolve_mode_ai_client(&state, Some(&mode_profile))
        .await
        .map_err(internal_error)?
        .unwrap_or_else(|| state.ai_client.clone());

    let reply = runtime_ai_client
        .chat(&ai_messages)
        .await
        .map_err(internal_error)?;

    if memory_enabled {
        db::append_message_pair(&state.pool, conversation_id, user_message, &reply)
            .await
            .map_err(internal_error)?;
    }

    {
        let mut manager = state
            .context_manager
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        manager.record_exchange(conversation_id, user_message, &reply);
    }

    Ok(Json(ChatResponse {
        conversation_id,
        mode,
        reply,
    }))
}

pub async fn chat_stream(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ChatRequest>,
) -> Result<Response, (StatusCode, String)> {
    let user_message = payload.message.trim();
    if user_message.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "message is empty".to_string()));
    }

    let mode = payload.mode.unwrap_or_default();
    let (mode_profile, conversation_id) = db::resolve_mode_profile_and_conversation(&state.pool, mode)
        .await
        .map_err(internal_error)?;

    let memory_enabled = profile_memory_enabled(Some(&mode_profile));
    let context_limit = profile_context_limit(Some(&mode_profile), state.chat_history_limit as usize);

    if memory_enabled {
        ensure_memory_cache_for_context(&state, conversation_id, context_limit).await?;
    }

    let profile_prompt = {
        let prompt = mode_profile.system_prompt.trim();
        if prompt.is_empty() {
            None
        } else {
            Some(prompt.to_string())
        }
    };
    let system_prompt = profile_prompt
        .as_deref()
        .or(state.prompts.system_prompt.as_deref());

    let ai_messages: Vec<ChatMessage> = {
        let mut manager = state
            .context_manager
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        manager.prepare_chat_messages(
            conversation_id,
            user_message,
            system_prompt,
            state.prompts.context_prompt.as_deref(),
            context_limit,
        )
    };

    let runtime_client =
        super::runtime_ai::resolve_mode_openai_compat_client(&state, Some(&mode_profile))
            .await
            .map_err(internal_error)?;
    let stream_client = runtime_client.unwrap_or_else(|| state.default_openai_client.as_ref().clone());

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<Bytes, Infallible>>();
    let state_for_task = state.clone();
    let ai_messages_for_task = ai_messages.clone();
    let user_message_for_task = user_message.to_string();
    tokio::spawn(async move {
        let tx_for_delta = tx.clone();
        let stream_result = stream_client.chat_stream(&ai_messages_for_task, |delta| {
            let _ = tx_for_delta.send(Ok(Bytes::copy_from_slice(delta.as_bytes())));
        });

        match stream_result.await {
            Ok(reply) => {
                if memory_enabled {
                    if let Err(error) = db::append_message_pair(
                        &state_for_task.pool,
                        conversation_id,
                        &user_message_for_task,
                        &reply,
                    )
                    .await
                    {
                        tracing::error!("stream append_message_pair failed: {error}");
                    }
                }

                {
                    let mut manager = state_for_task
                        .context_manager
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    manager.record_exchange(conversation_id, &user_message_for_task, &reply);
                }
            }
            Err(error) => {
                tracing::error!("stream chat failed: {error}");
                let error_text = format!("\n请求失败：{error}");
                let _ = tx.send(Ok(Bytes::copy_from_slice(error_text.as_bytes())));
            }
        }
    });

    let body = Body::from_stream(UnboundedReceiverStream::new(rx));
    let mut response = body.into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));

    Ok(response)
}

pub async fn list_messages(
    State(state): State<Arc<AppState>>,
    Query(query): Query<MessageQuery>,
) -> Result<Json<MessageListResponse>, (StatusCode, String)> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let before_id = query.before_id;
    let mode = query.mode.unwrap_or_default();
    let (mode_profile, conversation_id) = db::resolve_mode_profile_and_conversation(&state.pool, mode)
        .await
        .map_err(internal_error)?;
    let memory_enabled = profile_memory_enabled(Some(&mode_profile));

    let context_limit = profile_context_limit(Some(&mode_profile), state.chat_history_limit as usize);

    let messages = if memory_enabled {
        ensure_memory_cache_for_history(
            &state,
            conversation_id,
            before_id,
            limit as usize,
            context_limit,
        )
        .await?
    } else {
        let mut manager = state
            .context_manager
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        manager.list_cached_messages_desc(conversation_id, limit as usize, before_id)
    };

    Ok(Json(MessageListResponse {
        conversation_id,
        messages,
    }))
}

async fn ensure_memory_cache_for_context(
    state: &Arc<AppState>,
    conversation_id: i64,
    needed: usize,
) -> Result<(), (StatusCode, String)> {
    let effective_needed = needed.clamp(1, 200);
    loop {
        let meta = {
            let mut manager = state
                .context_manager
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            manager.conversation_cache_meta(conversation_id)
        };

        if !meta.hydrated {
            let bootstrap_limit = effective_needed as i64;
            let latest = db::list_messages(&state.pool, conversation_id, bootstrap_limit)
                .await
                .map_err(internal_error)?;
            let exhausted = latest.len() < bootstrap_limit as usize;
            let mut manager = state
                .context_manager
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            manager.hydrate_conversation_latest_from_db(conversation_id, latest, exhausted);
            continue;
        }

        if meta.cached_len >= effective_needed || meta.db_exhausted {
            return Ok(());
        }

        let Some(oldest_id) = meta.oldest_cached_id else {
            return Ok(());
        };
        let pull_limit = effective_needed
            .saturating_sub(meta.cached_len)
            .max(HISTORY_PAGE_SIZE as usize)
            .min(200) as i64;
        let older =
            db::list_messages_before_id(&state.pool, conversation_id, oldest_id, pull_limit)
            .await
            .map_err(internal_error)?;
        let exhausted = older.len() < pull_limit as usize;
        let is_empty = older.is_empty();
        let mut manager = state
            .context_manager
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let progressed = manager.prepend_conversation_older_from_db(conversation_id, older, exhausted);
        if is_empty || !progressed {
            return Ok(());
        }
    }
}

async fn ensure_memory_cache_for_history(
    state: &Arc<AppState>,
    conversation_id: i64,
    before_id: Option<i64>,
    limit: usize,
    preload_limit: usize,
) -> Result<Vec<db::StoredMessage>, (StatusCode, String)> {
    let effective_limit = limit.clamp(1, 200);
    let effective_preload = preload_limit.clamp(1, 200);

    loop {
        let (meta, cached) = {
            let mut manager = state
                .context_manager
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let meta = manager.conversation_cache_meta(conversation_id);
            let cached = manager.list_cached_messages_desc(conversation_id, effective_limit, before_id);
            (meta, cached)
        };

        if !meta.hydrated {
            let bootstrap_limit = effective_preload as i64;
            let latest = db::list_messages(&state.pool, conversation_id, bootstrap_limit)
                .await
                .map_err(internal_error)?;
            let exhausted = latest.len() < bootstrap_limit as usize;
            let mut manager = state
                .context_manager
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            manager.hydrate_conversation_latest_from_db(conversation_id, latest, exhausted);
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
        let older =
            db::list_messages_before_id(&state.pool, conversation_id, oldest_id, pull_limit)
            .await
            .map_err(internal_error)?;
        let exhausted = older.len() < pull_limit as usize;
        let is_empty = older.is_empty();
        let mut manager = state
            .context_manager
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let progressed = manager.prepend_conversation_older_from_db(conversation_id, older, exhausted);
        if is_empty || !progressed {
            return Ok(cached);
        }
    }
}
