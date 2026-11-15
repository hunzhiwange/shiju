//! 完整会话快照保存协调：合并、暂缓重试与永久失败隔离。
use session_ui_types::ChatSession;
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

#[cfg(not(target_arch = "wasm32"))]
struct PendingSessionSave {
    session: ChatSession,
    directory: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamSessionSaveResult {
    Committed,
    Coalesced,
    Deferred(String),
    Rejected(String),
    PermanentlyFailed(String),
    Blocked,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
pub struct StreamSessionSaveCoordinator {
    state: Mutex<StreamSessionSaveState>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct StreamSessionSaveState {
    sessions: HashMap<String, StreamSessionSaveSlot>,
    blocked_sessions: HashSet<String>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct StreamSessionSaveSlot {
    in_flight: bool,
    pending: Option<PendingSessionSave>,
}

#[cfg(not(target_arch = "wasm32"))]
impl StreamSessionSaveCoordinator {
    pub fn save<F>(
        &self,
        session: ChatSession,
        directory: Option<String>,
        mut save: F,
    ) -> StreamSessionSaveResult
    where
        F: FnMut(&ChatSession, Option<&str>) -> Result<Option<std::path::PathBuf>, String>,
    {
        let session_id = session.id.clone();
        let mut current = PendingSessionSave { session, directory };
        {
            let mut state = self.state.lock().expect("stream session save lock");
            if state.blocked_sessions.contains(&session_id) {
                return StreamSessionSaveResult::Blocked;
            }
            let slot = state.sessions.entry(session_id.clone()).or_default();
            if slot.in_flight {
                slot.pending = Some(current);
                return StreamSessionSaveResult::Coalesced;
            }
            slot.in_flight = true;
            slot.pending = None;
        }

        loop {
            let result = save(&current.session, current.directory.as_deref());
            if let Err(error) = result {
                let mut state = self.state.lock().expect("stream session save lock");
                if error.contains("session history conflict") {
                    if let Some(pending) =
                        state.sessions.get_mut(&session_id).and_then(|slot| slot.pending.take())
                    {
                        current = pending;
                        continue;
                    }
                    state.sessions.remove(&session_id);
                    return StreamSessionSaveResult::Rejected(error);
                }
                if crate::session_gateway::is_permanent_session_save_error(&error) {
                    state.blocked_sessions.insert(session_id.clone());
                    state.sessions.remove(&session_id);
                    tracing::warn!(
                        target: "vw_desktop",
                        session_id = %session_id,
                        "disabling stream session saves after permanent failure"
                    );
                    return StreamSessionSaveResult::PermanentlyFailed(error);
                }
                let slot = state
                    .sessions
                    .get_mut(&session_id)
                    .expect("stream session save slot should remain registered");
                slot.in_flight = false;
                if slot.pending.is_none() {
                    slot.pending = Some(current);
                }
                return StreamSessionSaveResult::Deferred(error);
            }
            let next = {
                let mut state = self.state.lock().expect("stream session save lock");
                let pending = state
                    .sessions
                    .get_mut(&session_id)
                    .expect("stream session save slot should remain registered")
                    .pending
                    .take();
                if pending.is_none() {
                    state.sessions.remove(&session_id);
                }
                pending
            };
            let Some(pending) = next else {
                return StreamSessionSaveResult::Committed;
            };
            current = pending;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub static STREAM_SESSION_SAVE_COORDINATOR: LazyLock<StreamSessionSaveCoordinator> =
    LazyLock::new(StreamSessionSaveCoordinator::default);

#[cfg(test)]
mod tests;

/// 保存结果到壳通知的唯一映射。
pub enum SaveNotification { Ack, None, Failed(String) }
pub fn save_notification(completed: Option<StreamSessionSaveResult>) -> SaveNotification {
    match completed {
        Some(StreamSessionSaveResult::Committed) => SaveNotification::Ack,
        Some(StreamSessionSaveResult::Coalesced | StreamSessionSaveResult::Deferred(_) | StreamSessionSaveResult::Blocked) => SaveNotification::None,
        Some(StreamSessionSaveResult::Rejected(error) | StreamSessionSaveResult::PermanentlyFailed(error)) => SaveNotification::Failed(error),
        None => {
            tracing::warn!(target: "vw_desktop", "stream session save task failed");
            SaveNotification::Failed("会话保存任务异常终止".to_string())
        }
    }
}
