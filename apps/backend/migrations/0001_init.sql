-- desktop-ai backend canonical schema (single-shot init, latest baseline).

-- ai_providers: runtime model provider registry.
CREATE TABLE IF NOT EXISTS ai_providers (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  provider_type TEXT NOT NULL,
  base_url TEXT,
  model_name TEXT NOT NULL,
  api_key_ref TEXT,
  enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
  temperature REAL,
  max_tokens INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ai_providers_is_default
ON ai_providers(is_default);

-- profiles: prompt/avatar presets for each mode.
CREATE TABLE IF NOT EXISTS profiles (
  id TEXT PRIMARY KEY,
  mode TEXT NOT NULL CHECK (mode IN ('default', 'roleplay')),
  name TEXT NOT NULL,
  avatar_path TEXT,
  system_prompt TEXT NOT NULL,
  opening_message TEXT,
  context_limit INTEGER NOT NULL DEFAULT 12,
  memory_enabled INTEGER NOT NULL DEFAULT 0 CHECK (memory_enabled IN (0, 1)),
  provider_id TEXT,
  extra_json TEXT,
  active_conversation_id INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY (provider_id) REFERENCES ai_providers(id),
  FOREIGN KEY (active_conversation_id) REFERENCES conversations(id),
  CHECK (
    (mode = 'default' AND memory_enabled = 0)
    OR (mode = 'roleplay' AND memory_enabled = 1)
  )
);

CREATE INDEX IF NOT EXISTS idx_profiles_mode
ON profiles(mode);

CREATE INDEX IF NOT EXISTS idx_profiles_provider_id
ON profiles(provider_id);

CREATE INDEX IF NOT EXISTS idx_profiles_active_conversation_id
ON profiles(active_conversation_id);

-- conversations: conversation-level metadata.
CREATE TABLE IF NOT EXISTS conversations (
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

CREATE INDEX IF NOT EXISTS idx_conversations_profile_updated_at
ON conversations(profile_id, updated_at DESC);

-- messages: ordered message stream under conversations.
CREATE TABLE IF NOT EXISTS messages (
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
  FOREIGN KEY (conversation_id) REFERENCES conversations(id),
  FOREIGN KEY (provider_id) REFERENCES ai_providers(id)
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation_seq
ON messages(conversation_id, seq);
