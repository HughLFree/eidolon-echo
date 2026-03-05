//! HTTP handlers for health check, chat, history list and OpenAPI output.

use crate::{ai::ChatMessage, db, AppState};
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub session_id: Option<i64>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub session_id: i64,
    pub reply: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    pub limit: Option<i64>,
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
    if payload.message.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "message is empty".to_string()));
    }

    let session_id = match payload.session_id {
        Some(v) => v,
        None => db::create_session(&state.pool)
            .await
            .map_err(internal_error)?,
    };

    db::save_message(&state.pool, session_id, "user", payload.message.trim())
        .await
        .map_err(internal_error)?;

    let history = db::list_messages(&state.pool, session_id, 30)
        .await
        .map_err(internal_error)?;

    let ai_messages: Vec<ChatMessage> = history
        .iter()
        .map(|m| ChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    let reply = state
        .ai_client
        .chat(&ai_messages)
        .await
        .map_err(internal_error)?;

    db::save_message(&state.pool, session_id, "assistant", &reply)
        .await
        .map_err(internal_error)?;

    Ok(Json(ChatResponse { session_id, reply }))
}

pub async fn list_messages(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<i64>,
    Query(query): Query<MessageQuery>,
) -> Result<Json<MessageListResponse>, (StatusCode, String)> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let messages = db::list_messages(&state.pool, session_id, limit)
        .await
        .map_err(internal_error)?;

    Ok(Json(MessageListResponse {
        session_id,
        messages,
    }))
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
