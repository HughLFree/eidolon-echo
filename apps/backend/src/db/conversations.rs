use super::{AiRoleMode, Profile};
use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;

pub async fn resolve_mode_profile_and_conversation(
    pool: &SqlitePool,
    mode: AiRoleMode,
) -> Result<(Profile, i64)> {
    let profile_id = super::profiles::ensure_mode_profile(pool, mode).await?;
    let profile = super::profiles::get_profile(pool, &profile_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("profile '{profile_id}' not found after ensure"))?;
    create_initial_conversation_for_profile(pool, &profile.id, mode, &profile.name).await?;
    let conversation_id = latest_conversation_id_for_profile(pool, &profile.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("no active conversation bound to profile '{}'", profile.id))?;
    Ok((profile, conversation_id))
}

pub(crate) async fn create_initial_conversation_for_profile(
    pool: &SqlitePool,
    profile_id: &str,
    mode: AiRoleMode,
    profile_name: &str,
) -> Result<()> {
    let exists: i64 = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM conversations WHERE profile_id = ? AND deleted_at IS NULL)",
    )
    .bind(profile_id)
    .fetch_one(pool)
    .await?;
    if exists != 0 {
        return Ok(());
    }

    let conversation_id = next_conversation_id(pool).await?;

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
    .execute(pool)
    .await?;

    Ok(())
}

async fn latest_conversation_id_for_profile(pool: &SqlitePool, profile_id: &str) -> Result<Option<i64>> {
    let conversation_id = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT id
        FROM conversations
        WHERE profile_id = ? AND deleted_at IS NULL
        ORDER BY updated_at DESC, id ASC
        LIMIT 1
        "#,
    )
    .bind(profile_id)
    .fetch_optional(pool)
    .await?;
    Ok(conversation_id)
}

async fn next_conversation_id(pool: &SqlitePool) -> Result<i64> {
    let next_id: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) + 1 FROM conversations")
            .fetch_one(pool)
            .await?;
    Ok(next_id.max(1))
}
