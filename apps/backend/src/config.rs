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
    pub providers: HashMap<String, ProviderConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub api_key_env: Option<String>,
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        load_env_files();

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

        let path = config_path.context("failed to locate config file")?;
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config file: {path}"))?;
        let cfg: AppConfig = toml::from_str(&raw).context("invalid TOML config")?;

        if !cfg.ai.providers.contains_key(&cfg.ai.default_provider) {
            bail!(
                "default provider '{}' not found in ai.providers",
                cfg.ai.default_provider
            );
        }
        if cfg.ai.chat_history_limit <= 0 {
            bail!("ai.chat_history_limit must be greater than 0");
        }
        for (provider_name, provider_cfg) in &cfg.ai.providers {
            if !(0.0..=2.0).contains(&provider_cfg.temperature) {
                bail!(
                    "ai.providers.{provider_name}.temperature must be between 0.0 and 2.0"
                );
            }
            if matches!(provider_cfg.max_tokens, Some(0)) {
                bail!("ai.providers.{provider_name}.max_tokens must be greater than 0");
            }
        }

        Ok(cfg)
    }
}

fn default_chat_history_limit() -> i64 {
    12
}

fn default_temperature() -> f32 {
    0.7
}

fn load_env_files() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let local_env = format!("{manifest_dir}/.env.local");
    let default_env = format!("{manifest_dir}/.env");
    let candidates = [
        ".env.local",
        ".env",
        "apps/backend/.env.local",
        "apps/backend/.env",
        local_env.as_str(),
        default_env.as_str(),
    ];

    for path in candidates {
        if Path::new(path).exists() {
            let _ = dotenvy::from_path(path);
        }
    }
}

impl ProviderConfig {
    pub fn resolved_api_key(&self, provider_name: &str) -> Result<String> {
        let direct = self.api_key.trim();
        if !direct.is_empty() {
            return Ok(direct.to_string());
        }

        if let Some(env_name) = self.api_key_env.as_deref() {
            let env_name = env_name.trim();
            if !env_name.is_empty() {
                return env::var(env_name).with_context(|| {
                    format!("missing env var '{env_name}' for provider '{provider_name}'")
                });
            }
        }

        let fallback = format!(
            "{}_API_KEY",
            provider_name
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect::<String>()
                .to_uppercase()
        );

        env::var(&fallback).with_context(|| {
            format!(
                "missing API key for provider '{provider_name}'; set 'api_key', 'api_key_env', or env '{fallback}'"
            )
        })
    }
}
