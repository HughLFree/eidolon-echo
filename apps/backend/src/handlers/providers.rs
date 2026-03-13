use super::{internal_error, sanitize_non_empty, sanitize_optional, DeleteResponse};
use crate::{db, AppState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct AiProviderQuery {
    pub with_disabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAiProviderRequest {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub base_url: Option<String>,
    pub model_name: String,
    pub api_key_ref: Option<String>,
    pub enabled: Option<bool>,
    pub is_default: Option<bool>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAiProviderRequest {
    pub name: String,
    pub provider_type: String,
    pub base_url: Option<String>,
    pub model_name: String,
    pub api_key_ref: Option<String>,
    pub enabled: bool,
    pub is_default: bool,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
}

pub async fn list_ai_providers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AiProviderQuery>,
) -> Result<Json<Vec<db::AiProvider>>, (StatusCode, String)> {
    let mut providers = db::list_ai_providers(&state.pool)
        .await
        .map_err(internal_error)?;

    if !query.with_disabled.unwrap_or(false) {
        providers.retain(|item| item.enabled);
    }
    Ok(Json(providers))
}

pub async fn get_ai_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<db::AiProvider>, (StatusCode, String)> {
    let id = sanitize_non_empty(&id, "id")?;
    let provider = db::get_ai_provider(&state.pool, &id)
        .await
        .map_err(internal_error)?;

    match provider {
        Some(item) => Ok(Json(item)),
        None => Err((StatusCode::NOT_FOUND, "provider not found".to_string())),
    }
}

pub async fn create_ai_provider(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateAiProviderRequest>,
) -> Result<(StatusCode, Json<db::AiProvider>), (StatusCode, String)> {
    let id = sanitize_non_empty(&payload.id, "id")?;
    if db::get_ai_provider(&state.pool, &id)
        .await
        .map_err(internal_error)?
        .is_some()
    {
        return Err((
            StatusCode::CONFLICT,
            "provider id already exists".to_string(),
        ));
    }

    let upsert = db::AiProviderUpsert {
        name: sanitize_non_empty(&payload.name, "name")?,
        provider_type: sanitize_non_empty(&payload.provider_type, "provider_type")?,
        base_url: sanitize_optional(payload.base_url),
        model_name: sanitize_non_empty(&payload.model_name, "model_name")?,
        api_key_ref: sanitize_optional(payload.api_key_ref),
        enabled: payload.enabled.unwrap_or(true),
        is_default: payload.is_default.unwrap_or(false),
        temperature: payload.temperature,
        max_tokens: normalize_optional_max_tokens(payload.max_tokens)?,
    };

    let provider = db::create_ai_provider(&state.pool, &id, &upsert)
        .await
        .map_err(internal_error)?;
    Ok((StatusCode::CREATED, Json(provider)))
}

pub async fn update_ai_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateAiProviderRequest>,
) -> Result<Json<db::AiProvider>, (StatusCode, String)> {
    let id = sanitize_non_empty(&id, "id")?;
    let upsert = db::AiProviderUpsert {
        name: sanitize_non_empty(&payload.name, "name")?,
        provider_type: sanitize_non_empty(&payload.provider_type, "provider_type")?,
        base_url: sanitize_optional(payload.base_url),
        model_name: sanitize_non_empty(&payload.model_name, "model_name")?,
        api_key_ref: sanitize_optional(payload.api_key_ref),
        enabled: payload.enabled,
        is_default: payload.is_default,
        temperature: payload.temperature,
        max_tokens: normalize_optional_max_tokens(payload.max_tokens)?,
    };

    let updated = db::update_ai_provider(&state.pool, &id, &upsert)
        .await
        .map_err(internal_error)?;
    match updated {
        Some(item) => Ok(Json(item)),
        None => Err((StatusCode::NOT_FOUND, "provider not found".to_string())),
    }
}

pub async fn delete_ai_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DeleteResponse>, (StatusCode, String)> {
    let id = sanitize_non_empty(&id, "id")?;
    let deleted = db::delete_ai_provider(&state.pool, &id)
        .await
        .map_err(internal_error)?;
    if !deleted {
        return Err((StatusCode::NOT_FOUND, "provider not found".to_string()));
    }
    Ok(Json(DeleteResponse { deleted }))
}

fn normalize_optional_max_tokens(raw: Option<i64>) -> Result<Option<i64>, (StatusCode, String)> {
    let Some(value) = raw else {
        return Ok(None);
    };
    if value <= 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "max_tokens must be > 0".to_string(),
        ));
    }
    Ok(Some(value.min(32768)))
}
