//! SQLite access layer: pool initialization, migrations and message/session queries.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row, SqlitePool,
};
use std::{path::Path, str::FromStr};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Clone, Serialize)]
pub struct StoredMessage {
    pub id: i64,
    pub session_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

pub async fn init_pool(path: &str) -> Result<SqlitePool> {
    if let Some(parent) = Path::new(path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let database_url = if path.starts_with("sqlite:") {
        path.to_string()
    } else {
        format!("sqlite://{path}")
    };
    let options = SqliteConnectOptions::from_str(&database_url)?.create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

#[allow(dead_code)]
pub async fn create_session(pool: &SqlitePool) -> Result<i64> {
    let res = sqlx::query("INSERT INTO sessions (title) VALUES (?)")
        .bind("新会话")
        .execute(pool)
        .await?;
    Ok(res.last_insert_rowid())
}

#[allow(dead_code)]
pub async fn save_message_pair(
    pool: &SqlitePool,
    session_id: i64,
    user_content: &str,
    assistant_content: &str,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query("INSERT INTO messages (session_id, role, content) VALUES (?, ?, ?)")
        .bind(session_id)
        .bind("user")
        .bind(user_content)
        .execute(&mut *tx)
        .await?;

    sqlx::query("INSERT INTO messages (session_id, role, content) VALUES (?, ?, ?)")
        .bind(session_id)
        .bind("assistant")
        .bind(assistant_content)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE sessions SET updated_at = datetime('now') WHERE id = ?")
        .bind(session_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn list_messages(
    pool: &SqlitePool,
    session_id: i64,
    limit: i64,
) -> Result<Vec<StoredMessage>> {
    let rows = sqlx::query(
        r#"
        SELECT id, session_id, role, content, created_at
        FROM messages
        WHERE session_id = ?
        ORDER BY id DESC
        LIMIT ?
        "#,
    )
    .bind(session_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut messages = Vec::with_capacity(rows.len());
    for row in rows {
        let created_at_raw: String = row.get("created_at");
        let dt = DateTime::parse_from_rfc3339(&format!("{}Z", created_at_raw.replace(' ', "T")))
            .map(|v| v.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        messages.push(StoredMessage {
            id: row.get("id"),
            session_id: row.get("session_id"),
            role: row.get("role"),
            content: row.get("content"),
            created_at: dt,
        });
    }

    messages.reverse();
    Ok(messages)
}

pub async fn list_messages_before_id(
    pool: &SqlitePool,
    session_id: i64,
    before_id: i64,
    limit: i64,
) -> Result<Vec<StoredMessage>> {
    let rows = sqlx::query(
        r#"
        SELECT id, session_id, role, content, created_at
        FROM messages
        WHERE session_id = ? AND id < ?
        ORDER BY id DESC
        LIMIT ?
        "#,
    )
    .bind(session_id)
    .bind(before_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut messages = Vec::with_capacity(rows.len());
    for row in rows {
        let created_at_raw: String = row.get("created_at");
        let dt = DateTime::parse_from_rfc3339(&format!("{}Z", created_at_raw.replace(' ', "T")))
            .map(|v| v.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        messages.push(StoredMessage {
            id: row.get("id"),
            session_id: row.get("session_id"),
            role: row.get("role"),
            content: row.get("content"),
            created_at: dt,
        });
    }

    messages.reverse();
    Ok(messages)
}
