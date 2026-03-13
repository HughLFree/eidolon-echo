use crate::{
    ai::{AiClient, OpenAiCompatClient},
    db, AppState,
};
use std::sync::Arc;

pub(super) async fn resolve_mode_ai_client(
    state: &Arc<AppState>,
    mode_profile: Option<&db::Profile>,
) -> anyhow::Result<Option<Arc<dyn AiClient>>> {
    let maybe_client = resolve_mode_openai_compat_client(state, mode_profile).await?;
    Ok(maybe_client.map(|client| Arc::new(client) as Arc<dyn AiClient>))
}

pub(super) async fn resolve_mode_openai_compat_client(
    state: &Arc<AppState>,
    mode_profile: Option<&db::Profile>,
) -> anyhow::Result<Option<OpenAiCompatClient>> {
    let provider = if let Some(profile) = mode_profile {
        if let Some(provider_id) = profile.provider_id.as_deref() {
            db::get_ai_provider(&state.pool, provider_id).await?
        } else {
            db::get_default_ai_provider(&state.pool).await?
        }
    } else {
        db::get_default_ai_provider(&state.pool).await?
    };

    let Some(provider) = provider else {
        return Ok(None);
    };
    if !provider.enabled {
        return Ok(None);
    }

    let provider_type = provider.provider_type.trim().to_lowercase();
    if provider_type != "openai_compat" && provider_type != "deepseek" && provider_type != "openai"
    {
        return Ok(None);
    }

    let base_url = provider
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("https://api.deepseek.com/v1")
        .to_string();

    let api_key = resolve_provider_api_key(provider.api_key.as_deref())?;
    let temperature = provider.temperature.unwrap_or(0.7).clamp(0.0, 2.0) as f32;
    let max_tokens = normalize_provider_max_tokens(provider.max_tokens);

    Ok(Some(OpenAiCompatClient::new(
        base_url,
        api_key,
        provider.model_name,
        temperature,
        max_tokens,
    )))
}

fn resolve_provider_api_key(raw: Option<&str>) -> anyhow::Result<String> {
    let api_key = raw
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("provider api_key is empty"))?;

    Ok(api_key.to_string())
}

fn normalize_provider_max_tokens(raw: Option<i64>) -> Option<u32> {
    let value = raw?;
    if value <= 0 {
        return None;
    }
    if value > u32::MAX as i64 {
        return Some(u32::MAX);
    }
    Some(value as u32)
}
