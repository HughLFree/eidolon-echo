use crate::{ai::AiRoleMode, db, AppState, StartupAiPrecheck};
use axum::{
    extract::{Query, State},
    http::header,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub ai_precheck: crate::StartupAiPrecheck,
}

#[derive(Debug, Deserialize)]
pub struct HealthQuery {
    pub mode: Option<AiRoleMode>,
}

pub async fn health(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HealthQuery>,
) -> Json<HealthResponse> {
    let ai_precheck = runtime_ai_precheck(&state, query.mode).await;
    Json(HealthResponse {
        status: "ok",
        ai_precheck,
    })
}

pub async fn openapi_yaml() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/yaml; charset=utf-8")],
        include_str!("../../openapi/openapi.yaml"),
    )
}

async fn runtime_ai_precheck(state: &Arc<AppState>, mode: Option<AiRoleMode>) -> StartupAiPrecheck {
    let mode = mode.unwrap_or_default();
    let mode_profile = match db::resolve_mode_profile_and_conversation(&state.pool, mode).await {
        Ok((profile, _)) => profile,
        Err(error) => {
            return StartupAiPrecheck {
                ready: false,
                provider: "runtime".to_string(),
                message: Some(format!("无法解析当前模式配置：{error}")),
            };
        }
    };

    let provider = match mode_profile.provider_id.as_deref() {
        Some(provider_id) => match db::get_ai_provider(&state.pool, provider_id).await {
            Ok(provider) => provider,
            Err(error) => {
                return StartupAiPrecheck {
                    ready: false,
                    provider: provider_id.to_string(),
                    message: Some(format!("读取当前模式 provider 失败：{error}")),
                };
            }
        },
        None => match db::get_default_ai_provider(&state.pool).await {
            Ok(provider) => provider,
            Err(error) => {
                return StartupAiPrecheck {
                    ready: false,
                    provider: "default".to_string(),
                    message: Some(format!("读取默认 provider 失败：{error}")),
                };
            }
        },
    };

    let Some(provider) = provider else {
        return StartupAiPrecheck {
            ready: false,
            provider: "runtime".to_string(),
            message: Some("当前模式未配置可用 provider，请先在“设置 -> API 设置”中保存模型配置。".to_string()),
        };
    };

    if !provider.enabled {
        return StartupAiPrecheck {
            ready: false,
            provider: provider.id,
            message: Some("当前模式绑定的 provider 已被禁用，请先在设置中启用或重新保存。".to_string()),
        };
    }

    let base_url = provider
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("https://api.deepseek.com/v1")
        .to_string();
    let api_key = provider.api_key.unwrap_or_default();

    crate::run_startup_ai_precheck(&provider.id, &base_url, &api_key, None).await
}
