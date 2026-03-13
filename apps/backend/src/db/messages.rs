use super::{rows_to_messages, StoredMessage};
use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;

pub async fn list_messages(
    pool: &SqlitePool,
    conversation_id: i64,
    limit: i64,
) -> Result<Vec<StoredMessage>> {
    let rows = sqlx::query(
        r#"
        SELECT seq AS id, conversation_id, role, content, created_at
        FROM messages
        WHERE conversation_id = ? AND deleted_at IS NULL
        ORDER BY seq DESC
        LIMIT ?
        "#,
    )
    .bind(conversation_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut messages = rows_to_messages(rows);
    messages.reverse();
    Ok(messages)
}

pub async fn list_messages_before_id(
    pool: &SqlitePool,
    conversation_id: i64,
    before_id: i64,
    limit: i64,
) -> Result<Vec<StoredMessage>> {
    let rows = sqlx::query(
        r#"
        SELECT seq AS id, conversation_id, role, content, created_at
        FROM messages
        WHERE conversation_id = ? AND seq < ? AND deleted_at IS NULL
        ORDER BY seq DESC
        LIMIT ?
        "#,
    )
    .bind(conversation_id)
    .bind(before_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut messages = rows_to_messages(rows);
    messages.reverse();
    Ok(messages)
}

pub async fn append_message_pair(
    pool: &SqlitePool,
    conversation_id: i64,
    user_content: &str,
    assistant_content: &str,
) -> Result<()> {
    let now = Utc::now().timestamp();
    let mut tx = pool.begin().await?;

    let base_seq: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(MAX(seq), 0)
        FROM messages
        WHERE conversation_id = ?
        "#,
    )
    .bind(conversation_id)
    .fetch_one(&mut *tx)
    .await?;

    let user_seq = base_seq + 1;
    let assistant_seq = base_seq + 2;
    let user_id = format!("{}-{}", conversation_id, user_seq);
    let assistant_id = format!("{}-{}", conversation_id, assistant_seq);

    sqlx::query(
        r#"
        INSERT INTO messages (
            id, conversation_id, role, content, seq, content_type, provider_id, created_at, updated_at, deleted_at
        ) VALUES (?, ?, ?, ?, ?, 'text', NULL, ?, ?, NULL)
        "#,
    )
    .bind(user_id)
    .bind(conversation_id)
    .bind("user")
    .bind(user_content)
    .bind(user_seq)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO messages (
            id, conversation_id, role, content, seq, content_type, provider_id, created_at, updated_at, deleted_at
        ) VALUES (?, ?, ?, ?, ?, 'text', NULL, ?, ?, NULL)
        "#,
    )
    .bind(assistant_id)
    .bind(conversation_id)
    .bind("assistant")
    .bind(assistant_content)
    .bind(assistant_seq)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE conversations SET updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(conversation_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}
