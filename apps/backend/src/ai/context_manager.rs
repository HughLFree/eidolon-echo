//! In-memory conversation context manager keyed by conversation id.

use super::ChatMessage;
use crate::db::StoredMessage;
use chrono::Utc;
use std::collections::{HashMap, VecDeque};
use tracing::warn;

const DEFAULT_MAX_MESSAGES_PER_CONVERSATION: usize = 100;
const DEFAULT_MAX_CACHED_CONVERSATIONS: usize = 128;

#[derive(Debug, Clone, Copy)]
pub struct ConversationCacheMeta {
    pub hydrated: bool,
    pub db_exhausted: bool,
    pub cached_len: usize,
    pub oldest_cached_id: Option<i64>,
}

#[derive(Debug, Clone)]
struct ConversationContext {
    messages: VecDeque<StoredMessage>,
    db_hydrated: bool,
    db_exhausted: bool,
}

impl ConversationContext {
    fn new() -> Self {
        Self {
            messages: VecDeque::new(),
            db_hydrated: false,
            db_exhausted: false,
        }
    }
}

#[derive(Debug)]
pub struct ContextManager {
    conversation_contexts: HashMap<i64, ConversationContext>,
    conversation_last_access: HashMap<i64, u64>,
    access_tick: u64,
    max_messages_per_conversation: usize,
    max_cached_conversations: usize,
    next_message_id: i64,
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new(
            DEFAULT_MAX_MESSAGES_PER_CONVERSATION,
            DEFAULT_MAX_CACHED_CONVERSATIONS,
        )
    }
}

impl ContextManager {
    pub fn new(max_messages_per_conversation: usize, max_cached_conversations: usize) -> Self {
        Self {
            conversation_contexts: HashMap::new(),
            conversation_last_access: HashMap::new(),
            access_tick: 0,
            max_messages_per_conversation: max_messages_per_conversation.max(1),
            max_cached_conversations: max_cached_conversations.max(1),
            next_message_id: 1,
        }
    }

    pub fn prepare_chat_messages(
        &mut self,
        conversation_id: i64,
        user_message: &str,
        system_prompt: Option<&str>,
        context_prompt: Option<&str>,
        context_history_limit: usize,
    ) -> Vec<ChatMessage> {
        let context = self.ensure_conversation_context(conversation_id);

        let mut messages = Vec::new();

        if let Some(system_prompt) = system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            });
        }

        if let Some(context_prompt) = context_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: context_prompt.to_string(),
            });
        }

        let history_window = context_history_limit.max(1);

        let start = context.messages.len().saturating_sub(history_window);
        for msg in context.messages.iter().skip(start) {
            messages.push(ChatMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        messages
    }

    pub fn record_exchange(
        &mut self,
        conversation_id: i64,
        user_content: &str,
        assistant_content: &str,
    ) {
        let user_id = self.next_message_id;
        self.next_message_id += 1;
        let assistant_id = self.next_message_id;
        self.next_message_id += 1;

        let max_messages_per_conversation = self.max_messages_per_conversation;
        let context = self.ensure_conversation_context(conversation_id);

        context.messages.push_back(StoredMessage {
            id: user_id,
            conversation_id,
            role: "user".to_string(),
            content: user_content.to_string(),
            created_at: Utc::now(),
        });
        context.messages.push_back(StoredMessage {
            id: assistant_id,
            conversation_id,
            role: "assistant".to_string(),
            content: assistant_content.to_string(),
            created_at: Utc::now(),
        });
        context.db_hydrated = true;
        if Self::trim_conversation_messages(context, max_messages_per_conversation) {
            // Oldest records may be reloaded from DB later after cap eviction.
            context.db_exhausted = false;
        }
    }

    /// Returns history ordered by newest -> oldest.
    pub fn list_cached_messages_desc(
        &mut self,
        conversation_id: i64,
        limit: usize,
        before_id: Option<i64>,
    ) -> Vec<StoredMessage> {
        if !self.conversation_contexts.contains_key(&conversation_id) {
            if self.conversation_last_access.contains_key(&conversation_id) {
                self.recover_missing_context(conversation_id, "list_cached_messages_desc");
            }
            return Vec::new();
        }
        self.touch_conversation_key(conversation_id);
        let Some(context) = self.conversation_context(conversation_id) else {
            self.recover_missing_context(conversation_id, "list_cached_messages_desc");
            return Vec::new();
        };

        context
            .messages
            .iter()
            .rev()
            .filter(|m| before_id.map(|cursor| m.id < cursor).unwrap_or(true))
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn conversation_cache_meta(&mut self, conversation_id: i64) -> ConversationCacheMeta {
        if !self.conversation_contexts.contains_key(&conversation_id) {
            if self.conversation_last_access.contains_key(&conversation_id) {
                self.recover_missing_context(conversation_id, "conversation_cache_meta");
            }
            return ConversationCacheMeta {
                hydrated: false,
                db_exhausted: false,
                cached_len: 0,
                oldest_cached_id: None,
            };
        }
        self.touch_conversation_key(conversation_id);
        let Some(context) = self.conversation_context(conversation_id) else {
            self.recover_missing_context(conversation_id, "conversation_cache_meta");
            return ConversationCacheMeta {
                hydrated: false,
                db_exhausted: false,
                cached_len: 0,
                oldest_cached_id: None,
            };
        };

        ConversationCacheMeta {
            hydrated: context.db_hydrated,
            db_exhausted: context.db_exhausted,
            cached_len: context.messages.len(),
            oldest_cached_id: context.messages.front().map(|m| m.id),
        }
    }

    /// Cold-start hydration from DB latest records.
    pub fn hydrate_conversation_latest_from_db(
        &mut self,
        conversation_id: i64,
        db_messages: Vec<StoredMessage>,
        db_exhausted: bool,
    ) {
        self.update_next_message_id_from_slice(&db_messages);
        let max_messages_per_conversation = self.max_messages_per_conversation;
        let context = self.ensure_conversation_context(conversation_id);
        if context.messages.is_empty() {
            context.messages = db_messages.into_iter().collect();
        }
        let trimmed = Self::trim_conversation_messages(context, max_messages_per_conversation);
        context.db_hydrated = true;
        // If we trimmed oldest entries due cap, keep DB backfill open.
        context.db_exhausted = db_exhausted && !trimmed;
    }

    /// Prepend older DB records into mode cache.
    pub fn prepend_conversation_older_from_db(
        &mut self,
        conversation_id: i64,
        db_messages: Vec<StoredMessage>,
        db_exhausted: bool,
    ) -> bool {
        self.update_next_message_id_from_slice(&db_messages);
        let max_messages_per_conversation = self.max_messages_per_conversation;
        let context = self.ensure_conversation_context(conversation_id);
        let oldest_before = context.messages.front().map(|m| m.id);
        for message in db_messages.into_iter().rev() {
            if context.messages.iter().any(|m| m.id == message.id) {
                continue;
            }
            context.messages.push_front(message);
        }
        let trimmed = Self::trim_conversation_messages(context, max_messages_per_conversation);
        context.db_hydrated = true;
        if db_exhausted && !trimmed {
            context.db_exhausted = true;
        }
        context.messages.front().map(|m| m.id) != oldest_before
    }

    fn update_next_message_id_from_slice(&mut self, messages: &[StoredMessage]) {
        if let Some(max_id) = messages.iter().map(|m| m.id).max() {
            self.next_message_id = self.next_message_id.max(max_id.saturating_add(1));
        }
    }

    fn ensure_conversation_context(&mut self, conversation_id: i64) -> &mut ConversationContext {
        self.touch_conversation_key(conversation_id);
        self.evict_lru_conversations(Some(conversation_id));
        self.conversation_contexts
            .entry(conversation_id)
            .or_insert_with(ConversationContext::new)
    }

    fn trim_conversation_messages(
        context: &mut ConversationContext,
        max_messages_per_conversation: usize,
    ) -> bool {
        if context.messages.len() <= max_messages_per_conversation {
            return false;
        }
        let overflow = context.messages.len() - max_messages_per_conversation;
        for _ in 0..overflow {
            let _ = context.messages.pop_front();
        }
        true
    }

    fn touch_conversation_key(&mut self, key: i64) {
        self.access_tick = self.access_tick.wrapping_add(1);
        self.conversation_last_access.insert(key, self.access_tick);
    }

    fn evict_lru_conversations(&mut self, pinned: Option<i64>) {
        while self.conversation_contexts.len() > self.max_cached_conversations {
            let oldest_key = self
                .conversation_last_access
                .iter()
                .filter(|(key, _)| Some(**key) != pinned)
                .min_by_key(|(_, tick)| **tick)
                .map(|(key, _)| *key);
            let Some(key) = oldest_key else {
                break;
            };
            self.conversation_contexts.remove(&key);
            self.conversation_last_access.remove(&key);
        }
    }

    fn conversation_context(&self, conversation_id: i64) -> Option<&ConversationContext> {
        self.conversation_contexts.get(&conversation_id)
    }

    fn recover_missing_context(&mut self, conversation_id: i64, operation: &str) {
        warn!(
            conversation_id,
            operation,
            "context cache metadata was present but conversation context was missing; clearing stale access state"
        );
        self.conversation_last_access.remove(&conversation_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_cached_messages_desc_recovers_from_stale_access_state() {
        let mut manager = ContextManager::default();
        manager.conversation_last_access.insert(42, 1);

        let messages = manager.list_cached_messages_desc(42, 10, None);

        assert!(messages.is_empty());
        assert!(!manager.conversation_last_access.contains_key(&42));
    }

    #[test]
    fn conversation_cache_meta_recovers_from_stale_access_state() {
        let mut manager = ContextManager::default();
        manager.conversation_last_access.insert(7, 1);

        let meta = manager.conversation_cache_meta(7);

        assert!(!meta.hydrated);
        assert_eq!(meta.cached_len, 0);
        assert!(!manager.conversation_last_access.contains_key(&7));
    }
}
