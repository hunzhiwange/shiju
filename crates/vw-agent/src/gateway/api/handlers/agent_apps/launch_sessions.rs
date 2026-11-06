//! Bounded, principal-scoped status for the actual execution-host CLI lifetime.
use axum::http::StatusCode;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};
use vw_api_types::{
    agent_apps::{LaunchSessionPhase, LaunchSessionStatus},
    id::TenantScope,
};
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Scope {
    pub tenant: TenantScope,
    pub host: String,
    pub principal: String,
}
#[derive(Clone, Default)]
pub(super) struct Sessions(Arc<Mutex<HashMap<String, (Scope, LaunchSessionPhase)>>>);
impl Sessions {
    pub fn start(
        &self,
        scope: Scope,
        work: impl FnOnce(mpsc::SyncSender<()>) -> Result<(), String> + Send + 'static,
    ) -> Result<String, StatusCode> {
        let id = uuid::Uuid::new_v4().to_string();
        {
            let mut entries = self.0.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            if entries.len() >= 128 {
                entries.retain(|_, (_, phase)| {
                    matches!(phase, LaunchSessionPhase::Starting | LaunchSessionPhase::Running)
                });
            }
            if entries.len() >= 128 {
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
            entries.insert(id.clone(), (scope, LaunchSessionPhase::Starting));
        }
        let sessions = self.clone();
        let worker_id = id.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        if std::thread::Builder::new()
            .name("app-terminal-session".into())
            .spawn(move || {
                let result = work(tx);
                if let Ok(mut entries) = sessions.0.lock()
                    && let Some((_, phase)) = entries.get_mut(&worker_id)
                {
                    *phase = if result.is_ok() {
                        LaunchSessionPhase::Completed
                    } else {
                        LaunchSessionPhase::Failed
                    };
                }
            })
            .is_err()
        {
            if let Ok(mut entries) = self.0.lock() {
                entries.remove(&id);
            }
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        // No retries: acknowledgement is only sent after the real CLI was spawned on its PTY.
        rx.recv_timeout(Duration::from_secs(35)).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        if let Ok(mut entries) = self.0.lock()
            && let Some((_, phase)) = entries.get_mut(&id)
            && *phase == LaunchSessionPhase::Starting
        {
            *phase = LaunchSessionPhase::Running;
        }
        Ok(id)
    }
    pub fn get(&self, scope: &Scope, id: &str) -> Result<LaunchSessionStatus, StatusCode> {
        let entries = self.0.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let (owner, phase) =
            entries.get(id).filter(|(owner, _)| owner == scope).ok_or(StatusCode::NOT_FOUND)?;
        let _ = owner;
        Ok(LaunchSessionStatus { phase: *phase })
    }
}
