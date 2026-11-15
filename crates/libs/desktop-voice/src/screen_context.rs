//! Explicit one-shot local window sharing. Tickets never survive disable/cancel/session changes.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub pid: i32,
    pub window: u32,
    pub label: String,
}
#[derive(Clone)]
pub struct Ticket {
    pub target: Target,
    live: Arc<AtomicBool>,
}
impl Ticket {
    pub fn capture<T>(
        &self,
        operation: impl FnOnce(&Target) -> Result<T, String>,
    ) -> Result<T, String> {
        if !self.is_live() {
            return Err("屏幕分享已取消。".into());
        }
        operation(&self.target)
    }
}
#[derive(Default)]
pub struct State {
    pub enabled: bool,
    pub offered: Option<Target>,
    pending: Option<Ticket>,
    delivered: bool,
    pub epoch: u64,
    pub selecting: bool,
    pub generation: Option<vw_api_voice::VoiceCounter>,
}
impl State {
    pub fn set_enabled(&mut self, enabled: bool) {
        self.cancel();
        self.enabled = enabled;
    }
    pub fn cancel(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        self.selecting = false;
        self.generation = None;
        self.delivered = false;
        if let Some(ticket) = self.pending.take() {
            ticket.live.store(false, Ordering::SeqCst);
        }
        self.offered = None;
    }
    pub fn begin(&mut self, target: Target, permission: bool) -> Option<Ticket> {
        self.cancel();
        if !self.enabled || !permission {
            return None;
        }
        let ticket = Ticket { target, live: Arc::new(AtomicBool::new(true)) };
        self.pending = Some(ticket.clone());
        Some(ticket)
    }
    pub fn owns(&self, ticket: &Ticket) -> bool {
        !self.delivered
            && self
                .pending
                .as_ref()
                .is_some_and(|p| Arc::ptr_eq(&p.live, &ticket.live) && p.target == ticket.target)
    }
    pub fn accept<T>(&mut self, ticket: &Ticket, frame: T, permission: bool) -> Option<T> {
        if !self.enabled
            || !permission
            || !ticket.is_live()
            || self.delivered
            || !self
                .pending
                .as_ref()
                .is_some_and(|p| Arc::ptr_eq(&p.live, &ticket.live) && p.target == ticket.target)
        {
            return None;
        }
        self.delivered = true;
        Some(frame)
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some() && !self.delivered
    }
}
impl Drop for State {
    fn drop(&mut self) {
        self.cancel();
    }
}
impl Ticket {
    pub fn cancel(&self) { self.live.store(false, Ordering::SeqCst); }
    pub fn is_live(&self) -> bool {
        self.live.load(Ordering::SeqCst)
    }
}

pub fn permitted() -> bool {
    use voice_permissions::{Permission, PermissionBackend, PermissionStatus, SystemPermissions};
    SystemPermissions.query(Permission::ScreenRecording) == PermissionStatus::Granted
}

#[cfg(test)]
#[path = "screen_context_tests.rs"]
mod tests;
