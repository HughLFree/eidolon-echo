-- Add explicit active conversation pointer on profiles.
-- The active pointer is nullable for compatibility; runtime bootstrap/fallback will fill it.

ALTER TABLE profiles
ADD COLUMN active_conversation_id INTEGER REFERENCES conversations(id);

-- Backfill existing profiles with their latest non-deleted conversation.
UPDATE profiles
SET active_conversation_id = (
  SELECT c.id
  FROM conversations c
  WHERE c.profile_id = profiles.id
    AND c.deleted_at IS NULL
  ORDER BY c.updated_at DESC, c.id ASC
  LIMIT 1
)
WHERE active_conversation_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_profiles_active_conversation_id
ON profiles(active_conversation_id);
