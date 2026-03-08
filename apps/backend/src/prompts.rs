//! System/context prompt loading for chat requests.

use anyhow::{Context, Result};
use std::{env, fs, path::Path};

#[derive(Debug, Clone, Default)]
pub struct PromptConfig {
    pub system_prompt: Option<String>,
    pub context_prompt: Option<String>,
}

impl PromptConfig {
    pub fn load() -> Result<Self> {
        let system_prompt = load_prompt(
            "SYSTEM_PROMPT_FILE",
            "prompts/system_prompt.local.md",
        )?;
        let context_prompt = load_prompt(
            "CONTEXT_PROMPT_FILE",
            "prompts/context_prompt.local.md",
        )?;

        Ok(Self {
            system_prompt,
            context_prompt,
        })
    }
}

fn load_prompt(env_key: &str, default_relative_path: &str) -> Result<Option<String>> {
    for path in candidate_paths(env_key, default_relative_path) {
        if !Path::new(&path).exists() {
            continue;
        }

        let content =
            fs::read_to_string(&path).with_context(|| format!("failed to read prompt file: {path}"))?;
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            return Ok(Some(trimmed.to_string()));
        }

        return Ok(None);
    }

    Ok(None)
}

fn candidate_paths(env_key: &str, default_relative_path: &str) -> Vec<String> {
    let mut candidates = Vec::new();

    if let Ok(explicit) = env::var(env_key) {
        let explicit = explicit.trim();
        if !explicit.is_empty() {
            candidates.push(explicit.to_string());
        }
    }

    candidates.push(default_relative_path.to_string());
    candidates.push(format!("apps/backend/{default_relative_path}"));
    candidates.push(format!(
        "{}/{}",
        env!("CARGO_MANIFEST_DIR"),
        default_relative_path
    ));

    candidates
}
