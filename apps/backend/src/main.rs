//! Backend service entrypoint: wires config, database, AI client and HTTP routes.

mod ai;
mod config;
mod db;
mod handlers;

use ai::{AiClient, ContextManager, OpenAiCompatClient};
use anyhow::{Context, Result};
use axum::{
    routing::{get, post},
    Router,
};
use config::AppConfig;
use reqwest::StatusCode as HttpStatusCode;
use sqlx::SqlitePool;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Clone, serde::Serialize)]
pub struct StartupAiPrecheck {
    pub ready: bool,
    pub provider: String,
    pub message: Option<String>,
}

pub struct AppState {
    pub pool: SqlitePool,
    pub ai_client: Arc<dyn AiClient>,
    pub default_openai_client: Arc<OpenAiCompatClient>,
    pub chat_history_limit: i64,
    pub context_manager: Mutex<ContextManager>,
    pub startup_ai_precheck: StartupAiPrecheck,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "desktop_ai_backend=info,tower_http=info".to_string()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = AppConfig::load()?;
    let pool = db::init_pool(&cfg.database.path).await?;
    db::bootstrap_mode_profiles_and_conversations(&pool, None).await?;

    let provider_cfg = cfg
        .ai
        .providers
        .get(&cfg.ai.default_provider)
        .context("missing provider config")?;
    let api_key_result = provider_cfg.resolved_api_key(&cfg.ai.default_provider);
    let key_resolve_error = api_key_result.as_ref().err().map(|err| err.to_string());
    let api_key = match api_key_result {
        Ok(key) => key,
        Err(error) => {
            tracing::warn!(
                "default provider key is missing; backend will start but default chat may fail until provider is configured: {error}"
            );
            String::new()
        }
    };

    let ai_client = OpenAiCompatClient::new(
        provider_cfg.base_url.clone(),
        api_key.clone(),
        provider_cfg.model.clone(),
        provider_cfg.temperature,
        provider_cfg.max_tokens,
    );
    let startup_ai_precheck = run_startup_ai_precheck(
        &cfg.ai.default_provider,
        &provider_cfg.base_url,
        &api_key,
        key_resolve_error.as_deref(),
    )
    .await;

    if !startup_ai_precheck.ready {
        if let Some(message) = startup_ai_precheck.message.as_deref() {
            tracing::warn!(
                "startup ai precheck failed for provider '{}': {}",
                startup_ai_precheck.provider,
                message
            );
        }
    }

    let state = Arc::new(AppState {
        pool,
        ai_client: Arc::new(ai_client.clone()),
        default_openai_client: Arc::new(ai_client),
        chat_history_limit: cfg.ai.chat_history_limit,
        context_manager: Mutex::new(ContextManager::new(
            cfg.ai.context_cache_max_messages_per_mode,
            cfg.ai.context_cache_max_modes,
        )),
        startup_ai_precheck,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    let app = Router::new()
        .route("/api/health", get(handlers::health))
        .route("/api/openapi.yaml", get(handlers::openapi_yaml))
        .route("/api/chat", post(handlers::chat))
        .route("/api/chat/stream", post(handlers::chat_stream))
        .route(
            "/api/ai-providers",
            get(handlers::list_ai_providers).post(handlers::create_ai_provider),
        )
        .route(
            "/api/ai-providers/:id",
            get(handlers::get_ai_provider)
                .put(handlers::update_ai_provider)
                .delete(handlers::delete_ai_provider),
        )
        .route(
            "/api/profiles",
            get(handlers::list_profiles).post(handlers::create_profile),
        )
        .route(
            "/api/profiles/:id",
            get(handlers::get_profile)
                .put(handlers::update_profile)
                .delete(handlers::delete_profile),
        )
        .route("/api/messages", get(handlers::list_messages))
        .with_state(state)
        .layer(cors);

    let addr: SocketAddr = format!("{}:{}", cfg.server.host, cfg.server.port).parse()?;
    tracing::info!("backend listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn run_startup_ai_precheck(
    provider: &str,
    base_url: &str,
    api_key: &str,
    key_resolve_error: Option<&str>,
) -> StartupAiPrecheck {
    if let Some(err) = key_resolve_error {
        return StartupAiPrecheck {
            ready: false,
            provider: provider.to_string(),
            message: Some(format!(
                "API key 未配置：{err}. 请在设置中填写 provider api_key_ref 或 api_key。"
            )),
        };
    }

    if api_key.trim().is_empty() {
        return StartupAiPrecheck {
            ready: false,
            provider: provider.to_string(),
            message: Some("API key 为空，请在设置中配置后再发送消息。".to_string()),
        };
    }

    let check_url = format!("{}/models", base_url.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            return StartupAiPrecheck {
                ready: false,
                provider: provider.to_string(),
                message: Some(format!("预检查初始化失败：{error}")),
            };
        }
    };

    let response = match client.get(&check_url).bearer_auth(api_key).send().await {
        Ok(response) => response,
        Err(error) => {
            return StartupAiPrecheck {
                ready: false,
                provider: provider.to_string(),
                message: Some(format!("无法连接模型服务（{check_url}）：{error}")),
            };
        }
    };

    if response.status().is_success() {
        return StartupAiPrecheck {
            ready: true,
            provider: provider.to_string(),
            message: None,
        };
    }

    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<empty body>".to_string());
    let message = if status == HttpStatusCode::UNAUTHORIZED || status == HttpStatusCode::FORBIDDEN {
        "鉴权失败：API key 无效或无权限，请检查设置中的 API key / api_key_ref / base_url。"
            .to_string()
    } else {
        format!(
            "预检查失败：HTTP {status}, body: {}",
            compact_for_message(&body)
        )
    };

    StartupAiPrecheck {
        ready: false,
        provider: provider.to_string(),
        message: Some(message),
    }
}

fn compact_for_message(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() <= 280 {
        return trimmed.to_string();
    }
    format!("{}...", &trimmed[..280])
}
