//! Backend service entrypoint: wires config, database, AI client and HTTP routes.

mod ai;
mod config;
mod db;
mod handlers;

use ai::{AiClient, ContextManager, OpenAiCompatClient};
use anyhow::{Context, Result};
use axum::{
    http::{
        header::{ACCEPT, CONTENT_TYPE},
        HeaderValue, Method,
    },
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

    let app = build_app(state);

    let addr: SocketAddr = format!("{}:{}", cfg.server.host, cfg.server.port).parse()?;
    tracing::info!("backend listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn build_app(state: Arc<AppState>) -> Router {
    Router::new()
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
        .layer(build_cors_layer())
}

fn build_cors_layer() -> CorsLayer {
    if cfg!(debug_assertions) {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_headers(Any)
            .allow_methods(Any)
    } else {
        CorsLayer::new()
            .allow_origin([
                HeaderValue::from_static("http://tauri.localhost"),
                HeaderValue::from_static("tauri://localhost"),
                HeaderValue::from_static("http://127.0.0.1:1420"),
                HeaderValue::from_static("http://localhost:1420"),
            ])
            .allow_headers([ACCEPT, CONTENT_TYPE])
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ])
    }
}

pub(crate) async fn run_startup_ai_precheck(
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
                "API key 未配置：{err}. 请在设置中填写 provider api_key 或配置文件中的 api_key。"
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
    let message = map_startup_precheck_failure(status, &body);

    StartupAiPrecheck {
        ready: false,
        provider: provider.to_string(),
        message: Some(message),
    }
}

pub(crate) fn compact_for_message(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() <= 280 {
        return trimmed.to_string();
    }
    format!("{}...", &trimmed[..280])
}

pub(crate) fn map_startup_precheck_failure(status: HttpStatusCode, body: &str) -> String {
    if status == HttpStatusCode::UNAUTHORIZED || status == HttpStatusCode::FORBIDDEN {
        return "鉴权失败：API key 无效或无权限，请检查设置中的 api_key / base_url / model_name。"
            .to_string();
    }

    format!(
        "预检查失败：HTTP {status}, body: {}",
        compact_for_message(body)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use serde_json::{json, Value};
    use sqlx::Row;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    use tower::util::ServiceExt;

    #[derive(Clone)]
    struct StaticAiClient {
        reply: String,
    }

    #[async_trait::async_trait]
    impl AiClient for StaticAiClient {
        async fn chat(&self, _messages: &[ai::ChatMessage]) -> Result<String> {
            Ok(self.reply.clone())
        }
    }

    fn next_test_db_path(name: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(1);

        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "eidolon-echo-{name}-{}-{unique}.db",
            std::process::id()
        ))
    }

    async fn make_test_state(name: &str, reply: &str) -> Arc<AppState> {
        let db_path = next_test_db_path(name);
        let _ = std::fs::remove_file(&db_path);

        let pool = db::init_pool(db_path.to_str().expect("utf8 temp path"))
            .await
            .expect("init test pool");
        db::bootstrap_mode_profiles_and_conversations(&pool, None)
            .await
            .expect("bootstrap conversations");

        Arc::new(AppState {
            pool,
            ai_client: Arc::new(StaticAiClient {
                reply: reply.to_string(),
            }),
            default_openai_client: Arc::new(OpenAiCompatClient::new(
                "http://127.0.0.1:9".to_string(),
                String::new(),
                "test-model".to_string(),
                0.0,
                None,
            )),
            chat_history_limit: 12,
            context_manager: Mutex::new(ContextManager::new(100, 128)),
            startup_ai_precheck: StartupAiPrecheck {
                ready: true,
                provider: "test".to_string(),
                message: None,
            },
        })
    }

    async fn request_json(app: Router, request: Request<Body>) -> (StatusCode, Value) {
        let response = app.oneshot(request).await.expect("router response");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let parsed = serde_json::from_slice(&body).expect("json body");
        (status, parsed)
    }

    #[tokio::test]
    async fn first_start_creates_bootstrap_profiles_and_conversations() {
        let db_path = next_test_db_path("first-start");
        let _ = std::fs::remove_file(&db_path);

        let pool = db::init_pool(db_path.to_str().expect("utf8 temp path"))
            .await
            .expect("init pool");
        db::bootstrap_mode_profiles_and_conversations(&pool, None)
            .await
            .expect("bootstrap");

        assert!(db_path.exists(), "database file should be created on first start");

        let profile_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM profiles")
            .fetch_one(&pool)
            .await
            .expect("count profiles");
        let conversation_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM conversations")
            .fetch_one(&pool)
            .await
            .expect("count conversations");

        assert_eq!(profile_count, 2);
        assert_eq!(conversation_count, 2);
    }

    #[tokio::test]
    async fn restarting_existing_db_does_not_duplicate_bootstrap_rows() {
        let db_path = next_test_db_path("restart");
        let _ = std::fs::remove_file(&db_path);

        let pool = db::init_pool(db_path.to_str().expect("utf8 temp path"))
            .await
            .expect("init first pool");
        db::bootstrap_mode_profiles_and_conversations(&pool, None)
            .await
            .expect("bootstrap first");
        drop(pool);

        let pool = db::init_pool(db_path.to_str().expect("utf8 temp path"))
            .await
            .expect("init second pool");
        db::bootstrap_mode_profiles_and_conversations(&pool, None)
            .await
            .expect("bootstrap second");

        let profile_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM profiles")
            .fetch_one(&pool)
            .await
            .expect("count profiles");
        let conversation_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM conversations")
            .fetch_one(&pool)
            .await
            .expect("count conversations");

        assert_eq!(profile_count, 2);
        assert_eq!(conversation_count, 2);
    }

    #[tokio::test]
    async fn startup_precheck_reports_missing_key_cleanly() {
        let result = run_startup_ai_precheck(
            "deepseek",
            "https://api.deepseek.com/v1",
            "",
            Some("missing api_key"),
        )
        .await;

        assert!(!result.ready);
        assert_eq!(result.provider, "deepseek");
        assert!(
            result
                .message
                .as_deref()
                .unwrap_or_default()
                .contains("API key 未配置")
        );
    }

    #[tokio::test]
    async fn startup_precheck_reports_invalid_key_cleanly() {
        let result = map_startup_precheck_failure(
            StatusCode::UNAUTHORIZED,
            &json!({
                "error": {
                    "message": "Authentication Fails",
                    "type": "authentication_error"
                }
            })
            .to_string(),
        );

        assert_eq!(
            result,
            "鉴权失败：API key 无效或无权限，请检查设置中的 api_key / base_url / model_name。"
        );
    }

    #[tokio::test]
    async fn default_mode_chat_returns_reply() {
        let state = make_test_state("default-chat", "测试回复").await;
        let app = build_app(state);

        let (status, body) = request_json(
            app,
            Request::post("/api/chat")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "mode": "default",
                        "message": "你好"
                    })
                    .to_string(),
                ))
                .expect("build request"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["mode"], "default");
        assert_eq!(body["reply"], "测试回复");
        assert!(body["conversation_id"].as_i64().unwrap_or_default() > 0);
    }

    #[tokio::test]
    async fn roleplay_mode_history_can_be_loaded_after_chat() {
        let state = make_test_state("roleplay-history", "江湖回答").await;
        let app = build_app(state.clone());

        let chat_response = app
            .clone()
            .oneshot(
                Request::post("/api/chat")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "mode": "roleplay",
                            "message": "陆小凤在吗"
                        })
                        .to_string(),
                    ))
                    .expect("build request"),
            )
            .await
            .expect("chat response");
        assert_eq!(chat_response.status(), StatusCode::OK);

        let (status, body) = request_json(
            app,
            Request::get("/api/messages?mode=roleplay&limit=10")
                .body(Body::empty())
                .expect("build request"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let messages = body["messages"].as_array().expect("messages array");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "assistant");
        assert_eq!(messages[0]["content"], "江湖回答");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "陆小凤在吗");

        let stored_count: i64 =
            sqlx::query("SELECT COUNT(*) AS count FROM messages WHERE conversation_id = ?")
                .bind(body["conversation_id"].as_i64().expect("conversation id"))
                .fetch_one(&state.pool)
                .await
                .expect("fetch count")
                .get("count");
        assert_eq!(stored_count, 2);
    }
}
