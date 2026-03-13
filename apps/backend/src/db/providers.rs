use super::{bool_to_i64, row_to_ai_provider, AiProvider, AiProviderUpsert};
use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;

pub async fn list_ai_providers(pool: &SqlitePool) -> Result<Vec<AiProvider>> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, provider_type, base_url, model_name, api_key_ref, enabled, is_default, temperature, max_tokens, created_at, updated_at
        FROM ai_providers
        ORDER BY is_default DESC, updated_at DESC, id ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut providers = Vec::with_capacity(rows.len());
    for row in rows {
        providers.push(row_to_ai_provider(row)?);
    }
    Ok(providers)
}

pub async fn get_ai_provider(pool: &SqlitePool, id: &str) -> Result<Option<AiProvider>> {
    let row = sqlx::query(
        r#"
        SELECT id, name, provider_type, base_url, model_name, api_key_ref, enabled, is_default, temperature, max_tokens, created_at, updated_at
        FROM ai_providers
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(row_to_ai_provider).transpose()
}

pub async fn get_default_ai_provider(pool: &SqlitePool) -> Result<Option<AiProvider>> {
    let row = sqlx::query(
        r#"
        SELECT id, name, provider_type, base_url, model_name, api_key_ref, enabled, is_default, temperature, max_tokens, created_at, updated_at
        FROM ai_providers
        WHERE enabled = 1 AND is_default = 1
        ORDER BY updated_at DESC, id ASC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    row.map(row_to_ai_provider).transpose()
}

pub async fn ai_provider_exists(pool: &SqlitePool, id: &str) -> Result<bool> {
    let exists: i64 = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM ai_providers WHERE id = ?)")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(exists != 0)
}

pub async fn create_ai_provider(
    pool: &SqlitePool,
    id: &str,
    payload: &AiProviderUpsert,
) -> Result<AiProvider> {
    let now = Utc::now().timestamp();
    let mut tx = pool.begin().await?;

    if payload.is_default {
        sqlx::query("UPDATE ai_providers SET is_default = 0, updated_at = ? WHERE is_default = 1")
            .bind(now)
            .execute(&mut *tx)
            .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO ai_providers (
            id, name, provider_type, base_url, model_name, api_key_ref, enabled, is_default, temperature, max_tokens, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(&payload.name)
    .bind(&payload.provider_type)
    .bind(&payload.base_url)
    .bind(&payload.model_name)
    .bind(&payload.api_key_ref)
    .bind(bool_to_i64(payload.enabled))
    .bind(bool_to_i64(payload.is_default))
    .bind(payload.temperature)
    .bind(payload.max_tokens)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    get_ai_provider(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("ai provider inserted but not found"))
}

pub async fn update_ai_provider(
    pool: &SqlitePool,
    id: &str,
    payload: &AiProviderUpsert,
) -> Result<Option<AiProvider>> {
    let now = Utc::now().timestamp();
    let mut tx = pool.begin().await?;

    if payload.is_default {
        sqlx::query(
            "UPDATE ai_providers SET is_default = 0, updated_at = ? WHERE id <> ? AND is_default = 1",
        )
        .bind(now)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }

    let result = sqlx::query(
        r#"
        UPDATE ai_providers
        SET
            name = ?,
            provider_type = ?,
            base_url = ?,
            model_name = ?,
            api_key_ref = ?,
            enabled = ?,
            is_default = ?,
            temperature = ?,
            max_tokens = ?,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&payload.name)
    .bind(&payload.provider_type)
    .bind(&payload.base_url)
    .bind(&payload.model_name)
    .bind(&payload.api_key_ref)
    .bind(bool_to_i64(payload.enabled))
    .bind(bool_to_i64(payload.is_default))
    .bind(payload.temperature)
    .bind(payload.max_tokens)
    .bind(now)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        tx.rollback().await?;
        return Ok(None);
    }

    tx.commit().await?;
    get_ai_provider(pool, id).await
}

pub async fn delete_ai_provider(pool: &SqlitePool, id: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM ai_providers WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
