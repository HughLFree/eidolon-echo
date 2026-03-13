-- Convert conversations.id / messages.conversation_id from TEXT to INTEGER.
-- This keeps existing rows and is safe when stored ids are numeric strings.

PRAGMA foreign_keys = OFF;

BEGIN TRANSACTION;

CREATE TABLE conversations_new (
  id INTEGER PRIMARY KEY,
  profile_id TEXT NOT NULL,
  title TEXT,
  summary TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  archived_at INTEGER,
  deleted_at INTEGER,
  FOREIGN KEY (profile_id) REFERENCES profiles(id)
);

INSERT INTO conversations_new (
  id, profile_id, title, summary, created_at, updated_at, archived_at, deleted_at
)
SELECT
  CAST(id AS INTEGER),
  profile_id,
  title,
  summary,
  created_at,
  updated_at,
  archived_at,
  deleted_at
FROM conversations;

CREATE TABLE messages_new (
  id TEXT PRIMARY KEY,
  conversation_id INTEGER NOT NULL,
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  seq INTEGER NOT NULL,
  content_type TEXT NOT NULL DEFAULT 'text',
  provider_id TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  deleted_at INTEGER,
  FOREIGN KEY (conversation_id) REFERENCES conversations_new(id),
  FOREIGN KEY (provider_id) REFERENCES ai_providers(id)
);

INSERT INTO messages_new (
  id, conversation_id, role, content, seq, content_type, provider_id, created_at, updated_at, deleted_at
)
SELECT
  id,
  CAST(conversation_id AS INTEGER),
  role,
  content,
  seq,
  content_type,
  provider_id,
  created_at,
  updated_at,
  deleted_at
FROM messages;

DROP TABLE messages;
DROP TABLE conversations;

ALTER TABLE conversations_new RENAME TO conversations;
ALTER TABLE messages_new RENAME TO messages;

CREATE INDEX IF NOT EXISTS idx_conversations_profile_updated_at
ON conversations(profile_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_messages_conversation_seq
ON messages(conversation_id, seq);

COMMIT;

PRAGMA foreign_keys = ON;
