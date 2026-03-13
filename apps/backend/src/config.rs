//! Application configuration loading and API key resolution.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{collections::HashMap, env, fs, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub ai: AiConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AiConfig {
    pub default_provider: String,
    #[serde(default = "default_chat_history_limit")]
    pub chat_history_limit: i64,
    #[serde(default = "default_context_cache_max_messages_per_mode")]
    pub context_cache_max_messages_per_mode: usize,
    #[serde(default = "default_context_cache_max_modes")]
    pub context_cache_max_modes: usize,
    pub providers: HashMap<String, ProviderConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let explicit = env::var("APP_CONFIG").ok();
        let manifest_default = format!("{}/config/default.toml", env!("CARGO_MANIFEST_DIR"));
        let candidates = [
            explicit.as_deref(),
            Some("config/default.toml"),
            Some("apps/backend/config/default.toml"),
            Some(manifest_default.as_str()),
        ];

        let mut config_path = None;
        for candidate in candidates.into_iter().flatten() {
            if Path::new(candidate).exists() {
                config_path = Some(candidate.to_string());
                break;
            }
        }

        let mut cfg = if let Some(path) = config_path {
            let raw = fs::read_to_string(&path)
                .with_context(|| format!("failed to read config file: {path}"))?;
            toml::from_str(&raw).context("invalid TOML config")?
        } else {
            default_config()
        };

        apply_env_overrides(&mut cfg)?;

        if !cfg.ai.providers.contains_key(&cfg.ai.default_provider) {
            bail!(
                "default provider '{}' not found in ai.providers",
                cfg.ai.default_provider
            );
        }
        if cfg.ai.chat_history_limit <= 0 {
            bail!("ai.chat_history_limit must be greater than 0");
        }
        if cfg.ai.context_cache_max_messages_per_mode == 0 {
            bail!("ai.context_cache_max_messages_per_mode must be greater than 0");
        }
        if cfg.ai.context_cache_max_modes == 0 {
            bail!("ai.context_cache_max_modes must be greater than 0");
        }
        for (provider_name, provider_cfg) in &cfg.ai.providers {
            if !(0.0..=2.0).contains(&provider_cfg.temperature) {
                bail!("ai.providers.{provider_name}.temperature must be between 0.0 and 2.0");
            }
            if matches!(provider_cfg.max_tokens, Some(0)) {
                bail!("ai.providers.{provider_name}.max_tokens must be greater than 0");
            }
        }

        Ok(cfg)
    }
}

fn apply_env_overrides(cfg: &mut AppConfig) -> Result<()> {
    if let Ok(path) = env::var("DATABASE_PATH") {
        let path = path.trim();
        if !path.is_empty() {
            cfg.database.path = path.to_string();
        }
    }

    if let Ok(host) = env::var("SERVER_HOST") {
        let host = host.trim();
        if !host.is_empty() {
            cfg.server.host = host.to_string();
        }
    }

    if let Ok(port) = env::var("SERVER_PORT") {
        let port = port.trim();
        if !port.is_empty() {
            cfg.server.port = port
                .parse::<u16>()
                .with_context(|| format!("invalid SERVER_PORT value: {port}"))?;
        }
    }

    Ok(())
}

fn default_config() -> AppConfig {
    let mut providers = HashMap::new();
    providers.insert(
        "deepseek".to_string(),
        ProviderConfig {
            base_url: "https://api.deepseek.com/v1".to_string(),
            api_key: String::new(),
            model: "deepseek-chat".to_string(),
            temperature: 0.9,
            max_tokens: Some(512),
        },
    );
    providers.insert(
        "openai".to_string(),
        ProviderConfig {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "gpt-4o-mini".to_string(),
            temperature: 0.7,
            max_tokens: None,
        },
    );

    AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3001,
        },
        database: DatabaseConfig {
            path: "apps/backend/data/chat.db".to_string(),
        },
        ai: AiConfig {
            default_provider: "deepseek".to_string(),
            chat_history_limit: 12,
            context_cache_max_messages_per_mode: 100,
            context_cache_max_modes: 128,
            providers,
        },
    }
}

fn default_chat_history_limit() -> i64 {
    12
}

fn default_context_cache_max_messages_per_mode() -> usize {
    100
}

fn default_context_cache_max_modes() -> usize {
    128
}

fn default_temperature() -> f32 {
    0.7
}

impl ProviderConfig {
    pub fn resolved_api_key(&self, provider_name: &str) -> Result<String> {
        let direct = self.api_key.trim();
        if !direct.is_empty() {
            return Ok(direct.to_string());
        }

        bail!(
            "missing API key for provider '{provider_name}'; set 'api_key' in config or provider api_key_ref in database settings"
        )
    }
}
