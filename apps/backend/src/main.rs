//! Backend service entrypoint: wires config, database, AI client and HTTP routes.

mod ai;
mod config;
mod db;
mod handlers;
mod prompts;

use ai::{AiClient, ContextManager, OpenAiCompatClient};
use anyhow::{Context, Result};
use axum::{
    routing::{get, post},
    Router,
};
use config::AppConfig;
use prompts::PromptConfig;
use sqlx::SqlitePool;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct AppState {
    pub pool: SqlitePool,
    pub ai_client: Arc<dyn AiClient>,
    pub prompts: PromptConfig,
    pub chat_history_limit: i64,
    pub context_manager: Mutex<ContextManager>,
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

    let provider_cfg = cfg
        .ai
        .providers
        .get(&cfg.ai.default_provider)
        .context("missing provider config")?;
    let api_key = provider_cfg.resolved_api_key(&cfg.ai.default_provider)?;

    let ai_client = OpenAiCompatClient::new(
        provider_cfg.base_url.clone(),
        api_key,
        provider_cfg.model.clone(),
        provider_cfg.temperature,
        provider_cfg.max_tokens,
    );
    let prompts = PromptConfig::load()?;

    let state = Arc::new(AppState {
        pool,
        ai_client: Arc::new(ai_client),
        prompts,
        chat_history_limit: cfg.ai.chat_history_limit,
        context_manager: Mutex::new(ContextManager::new(
            cfg.ai.context_cache_max_messages_per_mode,
            cfg.ai.context_cache_max_modes,
        )),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    let app = Router::new()
        .route("/api/health", get(handlers::health))
        .route("/api/openapi.yaml", get(handlers::openapi_yaml))
        .route("/api/chat", post(handlers::chat))
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
