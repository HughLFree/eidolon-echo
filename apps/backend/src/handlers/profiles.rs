use super::{internal_error, sanitize_non_empty, sanitize_optional, DeleteResponse};
use crate::{ai::AiRoleMode, db, AppState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct ProfileListQuery {
    pub mode: Option<AiRoleMode>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProfileRequest {
    pub id: String,
    pub mode: AiRoleMode,
    pub name: String,
    pub avatar_path: Option<String>,
    pub system_prompt: String,
    pub opening_message: Option<String>,
    pub context_limit: Option<i64>,
    pub provider_id: Option<String>,
    pub extra_json: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub mode: AiRoleMode,
    pub name: String,
    pub avatar_path: Option<String>,
    pub system_prompt: String,
    pub opening_message: Option<String>,
    pub context_limit: i64,
    pub provider_id: Option<String>,
    pub extra_json: Option<String>,
}

pub async fn list_profiles(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProfileListQuery>,
) -> Result<Json<Vec<db::Profile>>, (StatusCode, String)> {
    let profiles = db::list_profiles(&state.pool, query.mode)
        .await
        .map_err(internal_error)?;
    Ok(Json(profiles))
}

pub async fn get_profile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<db::Profile>, (StatusCode, String)> {
    let id = sanitize_non_empty(&id, "id")?;
    let profile = db::get_profile(&state.pool, &id)
        .await
        .map_err(internal_error)?;

    match profile {
        Some(item) => Ok(Json(item)),
        None => Err((StatusCode::NOT_FOUND, "profile not found".to_string())),
    }
}

pub async fn create_profile(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateProfileRequest>,
) -> Result<(StatusCode, Json<db::Profile>), (StatusCode, String)> {
    let id = sanitize_non_empty(&payload.id, "id")?;
    if db::get_profile(&state.pool, &id)
        .await
        .map_err(internal_error)?
        .is_some()
    {
        return Err((
            StatusCode::CONFLICT,
            "profile id already exists".to_string(),
        ));
    }

    validate_provider_reference(&state, payload.provider_id.as_deref()).await?;
    let upsert = build_profile_upsert(
        payload.mode,
        payload.name,
        payload.avatar_path,
        payload.system_prompt,
        payload.opening_message,
        payload.context_limit.unwrap_or(12),
        payload.provider_id,
        payload.extra_json,
    )?;

    let profile = db::create_profile(&state.pool, &id, &upsert)
        .await
        .map_err(internal_error)?;
    Ok((StatusCode::CREATED, Json(profile)))
}

pub async fn update_profile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<db::Profile>, (StatusCode, String)> {
    let id = sanitize_non_empty(&id, "id")?;
    validate_provider_reference(&state, payload.provider_id.as_deref()).await?;
    let upsert = build_profile_upsert(
        payload.mode,
        payload.name,
        payload.avatar_path,
        payload.system_prompt,
        payload.opening_message,
        payload.context_limit,
        payload.provider_id,
        payload.extra_json,
    )?;

    let updated = db::update_profile(&state.pool, &id, &upsert)
        .await
        .map_err(internal_error)?;
    match updated {
        Some(item) => Ok(Json(item)),
        None => Err((StatusCode::NOT_FOUND, "profile not found".to_string())),
    }
}

pub async fn delete_profile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DeleteResponse>, (StatusCode, String)> {
    let id = sanitize_non_empty(&id, "id")?;
    let deleted = db::delete_profile(&state.pool, &id)
        .await
        .map_err(internal_error)?;
    if !deleted {
        return Err((StatusCode::NOT_FOUND, "profile not found".to_string()));
    }
    Ok(Json(DeleteResponse { deleted }))
}

fn mode_memory_enabled(mode: AiRoleMode) -> bool {
    match mode {
        AiRoleMode::Default => false,
        AiRoleMode::Roleplay => true,
    }
}

fn build_profile_upsert(
    mode: AiRoleMode,
    name: String,
    avatar_path: Option<String>,
    system_prompt: String,
    opening_message: Option<String>,
    context_limit: i64,
    provider_id: Option<String>,
    extra_json: Option<String>,
) -> Result<db::ProfileUpsert, (StatusCode, String)> {
    let context_limit = context_limit.clamp(1, 200);
    Ok(db::ProfileUpsert {
        mode,
        name: sanitize_non_empty(&name, "name")?,
        avatar_path: sanitize_optional(avatar_path),
        system_prompt: sanitize_non_empty(&system_prompt, "system_prompt")?,
        opening_message: sanitize_optional(opening_message),
        context_limit,
        memory_enabled: mode_memory_enabled(mode),
        provider_id: sanitize_optional(provider_id),
        extra_json: sanitize_optional(extra_json),
    })
}

async fn validate_provider_reference(
    state: &Arc<AppState>,
    provider_id: Option<&str>,
) -> Result<(), (StatusCode, String)> {
    let Some(provider_id) = provider_id else {
        return Ok(());
    };
    let provider_id = provider_id.trim();
    if provider_id.is_empty() {
        return Ok(());
    }

    let exists = db::ai_provider_exists(&state.pool, provider_id)
        .await
        .map_err(internal_error)?;
    if !exists {
        return Err((StatusCode::BAD_REQUEST, "provider_id not found".to_string()));
    }

    Ok(())
}
