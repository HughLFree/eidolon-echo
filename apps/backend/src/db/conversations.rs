use super::{mode_to_str, AiRoleMode, Profile};
use anyhow::{anyhow, Result};
use chrono::Utc;
use sqlx::{Row, Sqlite, SqlitePool, Transaction};

pub async fn bootstrap_mode_profiles_and_conversations(
    pool: &SqlitePool,
    default_system_prompt: Option<&str>,
) -> Result<()> {
    for mode in [AiRoleMode::Default, AiRoleMode::Roleplay] {
        let profile_id =
            super::profiles::ensure_mode_profile(pool, mode, default_system_prompt).await?;
        let profile = super::profiles::get_profile(pool, &profile_id)
            .await?
            .ok_or_else(|| anyhow!("profile '{profile_id}' not found after ensure"))?;
        ensure_profile_active_conversation(pool, &profile.id, mode, &profile.name).await?;
    }
    Ok(())
}

pub async fn resolve_mode_profile_and_conversation(
    pool: &SqlitePool,
    mode: AiRoleMode,
) -> Result<(Profile, i64)> {
    if let Some(resolved) = resolve_mode_profile_and_conversation_fast(pool, mode).await? {
        return Ok(resolved);
    }

    repair_mode_profile_and_active_conversation(pool, mode).await?;

    resolve_mode_profile_and_conversation_fast(pool, mode)
        .await?
        .ok_or_else(|| {
            anyhow!(
                "failed to resolve active conversation for mode {}",
                mode_to_str(mode)
            )
        })
}

pub(crate) async fn ensure_profile_active_conversation(
    pool: &SqlitePool,
    profile_id: &str,
    mode: AiRoleMode,
    profile_name: &str,
) -> Result<i64> {
    let mut tx = pool.begin().await?;

    if let Some(active) = current_valid_active_conversation_id_tx(&mut tx, profile_id).await? {
        tx.commit().await?;
        return Ok(active);
    }

    let conversation_id =
        if let Some(existing) = latest_conversation_id_for_profile_tx(&mut tx, profile_id).await? {
            existing
        } else {
            create_conversation_for_profile_tx(&mut tx, profile_id, mode, profile_name).await?
        };

    sqlx::query(
        r#"
        UPDATE profiles
        SET active_conversation_id = ?
        WHERE id = ?
        "#,
    )
    .bind(conversation_id)
    .bind(profile_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(conversation_id)
}

async fn resolve_mode_profile_and_conversation_fast(
    pool: &SqlitePool,
    mode: AiRoleMode,
) -> Result<Option<(Profile, i64)>> {
    let row = sqlx::query(
        r#"
        SELECT
            p.id, p.mode, p.name, p.avatar_path, p.system_prompt, p.opening_message,
            p.context_limit, p.memory_enabled, p.provider_id, p.extra_json, p.created_at, p.updated_at,
            c.id AS conversation_id
        FROM profiles p
        JOIN conversations c
          ON c.id = p.active_conversation_id
         AND c.profile_id = p.id
         AND c.deleted_at IS NULL
        WHERE p.mode = ?
        ORDER BY p.updated_at DESC, p.id ASC
        LIMIT 1
        "#,
    )
    .bind(mode_to_str(mode))
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let conversation_id: i64 = row.get("conversation_id");
    let profile = super::row_to_profile(row)?;
    Ok(Some((profile, conversation_id)))
}

async fn repair_mode_profile_and_active_conversation(
    pool: &SqlitePool,
    mode: AiRoleMode,
) -> Result<()> {
    let profile_id = super::profiles::ensure_mode_profile(pool, mode, None).await?;
    let profile = super::profiles::get_profile(pool, &profile_id)
        .await?
        .ok_or_else(|| anyhow!("profile '{profile_id}' not found after ensure"))?;
    ensure_profile_active_conversation(pool, &profile.id, mode, &profile.name).await?;
    Ok(())
}

async fn current_valid_active_conversation_id_tx(
    tx: &mut Transaction<'_, Sqlite>,
    profile_id: &str,
) -> Result<Option<i64>> {
    let active = sqlx::query_scalar(
        r#"
        SELECT c.id
        FROM profiles p
        JOIN conversations c
          ON c.id = p.active_conversation_id
         AND c.profile_id = p.id
         AND c.deleted_at IS NULL
        WHERE p.id = ?
        LIMIT 1
        "#,
    )
    .bind(profile_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(active)
}

async fn latest_conversation_id_for_profile_tx(
    tx: &mut Transaction<'_, Sqlite>,
    profile_id: &str,
) -> Result<Option<i64>> {
    let latest = sqlx::query_scalar(
        r#"
        SELECT id
        FROM conversations
        WHERE profile_id = ? AND deleted_at IS NULL
        ORDER BY updated_at DESC, id ASC
        LIMIT 1
        "#,
    )
    .bind(profile_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(latest)
}

async fn create_conversation_for_profile_tx(
    tx: &mut Transaction<'_, Sqlite>,
    profile_id: &str,
    mode: AiRoleMode,
    profile_name: &str,
) -> Result<i64> {
    let conversation_id = next_conversation_id_tx(tx).await?;
    let title = match mode {
        AiRoleMode::Default => "Default Conversation".to_string(),
        AiRoleMode::Roleplay => format!("{profile_name} Conversation"),
    };
    let now = Utc::now().timestamp();

    sqlx::query(
        r#"
        INSERT INTO conversations (
            id, profile_id, title, summary, created_at, updated_at, archived_at, deleted_at
        ) VALUES (?, ?, ?, NULL, ?, ?, NULL, NULL)
        "#,
    )
    .bind(conversation_id)
    .bind(profile_id)
    .bind(title)
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await?;

    Ok(conversation_id)
}

async fn next_conversation_id_tx(tx: &mut Transaction<'_, Sqlite>) -> Result<i64> {
    let next_id: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) + 1 FROM conversations")
        .fetch_one(&mut **tx)
        .await?;
    Ok(next_id.max(1))
}
