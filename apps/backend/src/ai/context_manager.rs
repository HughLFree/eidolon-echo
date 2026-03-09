//! In-memory conversation context manager for role modes.

use super::{AiRoleMode, ChatMessage};
use crate::db::StoredMessage;
use chrono::Utc;
use std::collections::{HashMap, VecDeque};

const DEFAULT_MODE_CONTEXT_WINDOW: usize = 10;

#[derive(Debug, Clone, Copy)]
pub struct RoleplayCacheMeta {
    pub hydrated: bool,
    pub db_exhausted: bool,
    pub cached_len: usize,
    pub oldest_cached_id: Option<i64>,
}

#[derive(Debug, Clone)]
struct SessionContext {
    messages: VecDeque<StoredMessage>,
    roleplay_hydrated: bool,
    roleplay_db_exhausted: bool,
}

impl SessionContext {
    fn new(mode: AiRoleMode) -> Self {
        let roleplay = mode == AiRoleMode::Roleplay;
        Self {
            messages: VecDeque::new(),
            roleplay_hydrated: !roleplay,
            roleplay_db_exhausted: !roleplay,
        }
    }
}

#[derive(Debug)]
pub struct ContextManager {
    sessions: HashMap<(i64, AiRoleMode), SessionContext>,
    next_session_id: i64,
    next_message_id: i64,
}

impl Default for ContextManager {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            next_session_id: 1_000_000,
            next_message_id: 1,
        }
    }
}

impl ContextManager {
    pub fn prepare_chat_messages(
        &mut self,
        session_id: Option<i64>,
        mode: AiRoleMode,
        user_message: &str,
        system_prompt: Option<&str>,
        context_prompt: Option<&str>,
        roleplay_history_limit: usize,
    ) -> (i64, Vec<ChatMessage>) {
        let sid = self.ensure_session_id(session_id);
        let session = self.ensure_session_context(sid, mode);

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

        let history_window = match mode {
            AiRoleMode::Default => DEFAULT_MODE_CONTEXT_WINDOW,
            AiRoleMode::Roleplay => roleplay_history_limit.saturating_mul(2),
        };

        let start = session.messages.len().saturating_sub(history_window);
        for msg in session.messages.iter().skip(start) {
            messages.push(ChatMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        (sid, messages)
    }

    pub fn record_exchange(
        &mut self,
        session_id: i64,
        mode: AiRoleMode,
        user_content: &str,
        assistant_content: &str,
    ) {
        let user_id = self.next_message_id;
        self.next_message_id += 1;
        let assistant_id = self.next_message_id;
        self.next_message_id += 1;

        let session = self.ensure_session_context(session_id, mode);

        session.messages.push_back(StoredMessage {
            id: user_id,
            session_id,
            role: "user".to_string(),
            content: user_content.to_string(),
            created_at: Utc::now(),
        });
        session.messages.push_back(StoredMessage {
            id: assistant_id,
            session_id,
            role: "assistant".to_string(),
            content: assistant_content.to_string(),
            created_at: Utc::now(),
        });
    }

    /// Returns history ordered by newest -> oldest.
    pub fn list_cached_messages_desc(
        &self,
        session_id: i64,
        mode: AiRoleMode,
        limit: usize,
        before_id: Option<i64>,
    ) -> Vec<StoredMessage> {
        let Some(session) = self.sessions.get(&(session_id, mode)) else {
            return Vec::new();
        };

        session
            .messages
            .iter()
            .rev()
            .filter(|m| before_id.map(|cursor| m.id < cursor).unwrap_or(true))
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn roleplay_cache_meta(&self, session_id: i64) -> RoleplayCacheMeta {
        let Some(session) = self.sessions.get(&(session_id, AiRoleMode::Roleplay)) else {
            return RoleplayCacheMeta {
                hydrated: false,
                db_exhausted: false,
                cached_len: 0,
                oldest_cached_id: None,
            };
        };

        RoleplayCacheMeta {
            hydrated: session.roleplay_hydrated,
            db_exhausted: session.roleplay_db_exhausted,
            cached_len: session.messages.len(),
            oldest_cached_id: session.messages.front().map(|m| m.id),
        }
    }

    /// Cold-start hydration from DB latest records.
    pub fn hydrate_roleplay_latest_from_db(
        &mut self,
        session_id: i64,
        db_messages: Vec<StoredMessage>,
        db_exhausted: bool,
    ) {
        self.update_next_message_id_from_slice(&db_messages);
        let session = self.ensure_session_context(session_id, AiRoleMode::Roleplay);
        if session.messages.is_empty() {
            session.messages = db_messages.into_iter().collect();
        }
        session.roleplay_hydrated = true;
        session.roleplay_db_exhausted = db_exhausted;
    }

    /// Prepend older DB records into roleplay cache.
    pub fn prepend_roleplay_older_from_db(
        &mut self,
        session_id: i64,
        db_messages: Vec<StoredMessage>,
        db_exhausted: bool,
    ) {
        self.update_next_message_id_from_slice(&db_messages);
        let session = self.ensure_session_context(session_id, AiRoleMode::Roleplay);
        for message in db_messages.into_iter().rev() {
            if session.messages.iter().any(|m| m.id == message.id) {
                continue;
            }
            session.messages.push_front(message);
        }
        session.roleplay_hydrated = true;
        if db_exhausted {
            session.roleplay_db_exhausted = true;
        }
    }

    pub fn reserve_roleplay_history_db_sync(&self, _session_id: i64) {
        // Reserved extension point:
        // Roleplay mode history can be mirrored to DB in a future iteration.
    }

    fn update_next_message_id_from_slice(&mut self, messages: &[StoredMessage]) {
        if let Some(max_id) = messages.iter().map(|m| m.id).max() {
            self.next_message_id = self.next_message_id.max(max_id.saturating_add(1));
        }
    }

    fn ensure_session_id(&mut self, session_id: Option<i64>) -> i64 {
        if let Some(sid) = session_id {
            return sid;
        }

        let sid = self.next_session_id;
        self.next_session_id += 1;
        sid
    }

    fn ensure_session_context(&mut self, session_id: i64, mode: AiRoleMode) -> &mut SessionContext {
        self.sessions
            .entry((session_id, mode))
            .or_insert_with(|| SessionContext::new(mode))
    }
}
