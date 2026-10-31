//! 单条 Agent 连接的初始化与会话绑定状态。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::backend::{BackendCancelHandle, BackendHistoryEntry};

/// 已验证且只属于当前连接的会话绑定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionBinding {
    backend_session_id: String,
    cwd: PathBuf,
    history: Vec<BackendHistoryEntry>,
}

impl SessionBinding {
    pub(crate) fn new(
        backend_session_id: String,
        cwd: PathBuf,
        history: Vec<BackendHistoryEntry>,
    ) -> Self {
        Self { backend_session_id, cwd, history }
    }

    /// 返回宿主原生会话 ID。
    pub fn backend_session_id(&self) -> &str {
        &self.backend_session_id
    }

    /// 返回规范化工作目录。
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// 返回已完成归属校验的历史。
    pub fn history(&self) -> &[BackendHistoryEntry] {
        &self.history
    }
}

struct ActivePrompt {
    cancel: Option<Arc<dyn BackendCancelHandle>>,
    cancel_requested: bool,
}

#[derive(Default)]
pub(crate) struct ConnectionState {
    initialized: bool,
    sessions: HashMap<String, SessionBinding>,
    active_prompts: HashMap<String, ActivePrompt>,
}

impl ConnectionState {
    pub(crate) fn initialize(&mut self) -> bool {
        if self.initialized {
            return false;
        }
        self.initialized = true;
        true
    }

    pub(crate) fn initialized(&self) -> bool {
        self.initialized
    }

    pub(crate) fn insert(
        &mut self,
        session_id: String,
        binding: SessionBinding,
    ) -> Result<(), SessionBinding> {
        if self.sessions.contains_key(&session_id) {
            return Err(binding);
        }
        self.sessions.insert(session_id, binding);
        Ok(())
    }

    pub(crate) fn binding(&self, session_id: &str) -> Option<SessionBinding> {
        self.sessions.get(session_id).cloned()
    }

    pub(crate) fn replace_binding(&mut self, session_id: String, binding: SessionBinding) {
        self.sessions.insert(session_id, binding);
    }

    pub(crate) fn reserve_prompt(&mut self, session_id: &str) -> bool {
        if self.active_prompts.contains_key(session_id) {
            return false;
        }
        self.active_prompts
            .insert(session_id.to_string(), ActivePrompt { cancel: None, cancel_requested: false });
        true
    }

    pub(crate) fn attach_cancel(&mut self, session_id: &str, cancel: Arc<dyn BackendCancelHandle>) {
        let Some(active) = self.active_prompts.get_mut(session_id) else {
            return;
        };
        if active.cancel_requested {
            cancel.cancel();
        }
        active.cancel = Some(cancel);
    }

    pub(crate) fn cancel_prompt(&mut self, session_id: &str) -> bool {
        let Some(active) = self.active_prompts.get_mut(session_id) else {
            return false;
        };
        if active.cancel_requested {
            return false;
        }
        active.cancel_requested = true;
        if let Some(cancel) = active.cancel.as_ref() {
            cancel.cancel();
        }
        true
    }

    pub(crate) fn finish_prompt(&mut self, session_id: &str) {
        self.active_prompts.remove(session_id);
    }
}
