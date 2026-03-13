use super::{
    bool_to_i64, mode_to_str, row_to_profile, AiRoleMode, Profile, ProfileUpsert,
    AUTO_DEFAULT_PROFILE_ID, AUTO_ROLEPLAY_PROFILE_ID,
};
use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;

pub async fn list_profiles(pool: &SqlitePool, mode: Option<AiRoleMode>) -> Result<Vec<Profile>> {
    let rows = if let Some(value) = mode {
        sqlx::query(
            r#"
            SELECT id, mode, name, avatar_path, system_prompt, opening_message, context_limit, memory_enabled, provider_id, extra_json, created_at, updated_at
            FROM profiles
            WHERE mode = ?
            ORDER BY updated_at DESC, id ASC
            "#,
        )
        .bind(mode_to_str(value))
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT id, mode, name, avatar_path, system_prompt, opening_message, context_limit, memory_enabled, provider_id, extra_json, created_at, updated_at
            FROM profiles
            ORDER BY updated_at DESC, id ASC
            "#,
        )
        .fetch_all(pool)
        .await?
    };

    let mut profiles = Vec::with_capacity(rows.len());
    for row in rows {
        profiles.push(row_to_profile(row)?);
    }
    Ok(profiles)
}

pub async fn latest_profile_by_mode(
    pool: &SqlitePool,
    mode: AiRoleMode,
) -> Result<Option<Profile>> {
    let row = sqlx::query(
        r#"
        SELECT id, mode, name, avatar_path, system_prompt, opening_message, context_limit, memory_enabled, provider_id, extra_json, created_at, updated_at
        FROM profiles
        WHERE mode = ?
        ORDER BY updated_at DESC, id ASC
        LIMIT 1
        "#,
    )
    .bind(mode_to_str(mode))
    .fetch_optional(pool)
    .await?;

    row.map(row_to_profile).transpose()
}

pub async fn get_profile(pool: &SqlitePool, id: &str) -> Result<Option<Profile>> {
    let row = sqlx::query(
        r#"
        SELECT id, mode, name, avatar_path, system_prompt, opening_message, context_limit, memory_enabled, provider_id, extra_json, created_at, updated_at
        FROM profiles
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(row_to_profile).transpose()
}

pub async fn create_profile(
    pool: &SqlitePool,
    id: &str,
    payload: &ProfileUpsert,
) -> Result<Profile> {
    let now = Utc::now().timestamp();

    sqlx::query(
        r#"
        INSERT INTO profiles (
            id, mode, name, avatar_path, system_prompt, opening_message, context_limit, memory_enabled, provider_id, extra_json, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(mode_to_str(payload.mode))
    .bind(&payload.name)
    .bind(&payload.avatar_path)
    .bind(&payload.system_prompt)
    .bind(&payload.opening_message)
    .bind(payload.context_limit)
    .bind(bool_to_i64(payload.memory_enabled))
    .bind(&payload.provider_id)
    .bind(&payload.extra_json)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    super::conversations::ensure_profile_active_conversation(pool, id, payload.mode, &payload.name)
        .await?;

    get_profile(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("profile inserted but not found"))
}

pub async fn update_profile(
    pool: &SqlitePool,
    id: &str,
    payload: &ProfileUpsert,
) -> Result<Option<Profile>> {
    let now = Utc::now().timestamp();
    let result = sqlx::query(
        r#"
        UPDATE profiles
        SET
            mode = ?,
            name = ?,
            avatar_path = ?,
            system_prompt = ?,
            opening_message = ?,
            context_limit = ?,
            memory_enabled = ?,
            provider_id = ?,
            extra_json = ?,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(mode_to_str(payload.mode))
    .bind(&payload.name)
    .bind(&payload.avatar_path)
    .bind(&payload.system_prompt)
    .bind(&payload.opening_message)
    .bind(payload.context_limit)
    .bind(bool_to_i64(payload.memory_enabled))
    .bind(&payload.provider_id)
    .bind(&payload.extra_json)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Ok(None);
    }

    get_profile(pool, id).await
}

pub async fn delete_profile(pool: &SqlitePool, id: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM profiles WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

const DEFAULT_MODE_SYSTEM_PROMPT: &str = "你是一个桌宠 AI 助手。\n\n要求：\n- 回答简洁、明确、可执行。\n- 优先使用中文回复。\n- 不要编造事实；不确定时明确说明不确定。";

pub(crate) async fn ensure_mode_profile(
    pool: &SqlitePool,
    mode: AiRoleMode,
    default_system_prompt: Option<&str>,
) -> Result<String> {
    if let Some(profile) = latest_profile_by_mode(pool, mode).await? {
        return Ok(profile.id);
    }

    let (id, name, system_prompt, memory_enabled) = match mode {
        AiRoleMode::Default => (
            AUTO_DEFAULT_PROFILE_ID,
            "Default Profile",
            default_system_prompt
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(DEFAULT_MODE_SYSTEM_PROMPT),
            0_i64,
        ),
        AiRoleMode::Roleplay => (
            AUTO_ROLEPLAY_PROFILE_ID,
            "Roleplay Profile",
            "Stay in character and answer naturally.",
            1_i64,
        ),
    };

    let now = Utc::now().timestamp();
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO profiles (
            id, mode, name, avatar_path, system_prompt, opening_message, context_limit, memory_enabled, provider_id, extra_json, created_at, updated_at
        ) VALUES (?, ?, ?, NULL, ?, NULL, 12, ?, NULL, NULL, ?, ?)
        "#,
    )
    .bind(id)
    .bind(mode_to_str(mode))
    .bind(name)
    .bind(system_prompt)
    .bind(memory_enabled)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    let profile = latest_profile_by_mode(pool, mode).await?.ok_or_else(|| {
        anyhow::anyhow!("failed to ensure profile for mode {}", mode_to_str(mode))
    })?;
    Ok(profile.id)
}
