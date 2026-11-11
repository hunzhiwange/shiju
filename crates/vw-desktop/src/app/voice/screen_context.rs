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
    #[cfg(any(target_os = "macos", test))]
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
pub(crate) struct State {
    pub enabled: bool,
    pub offered: Option<Target>,
    pending: Option<Ticket>,
    delivered: bool,
    pub epoch: u64,
    pub selecting: bool,
    pub generation: Option<vw_gateway_client::vw_api_types::voice::VoiceCounter>,
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
#[cfg(test)]
#[path = "screen_context_tests.rs"]
mod tests;

pub(crate) fn permitted() -> bool {
    use super::permissions::{Permission, PermissionBackend, PermissionStatus, SystemPermissions};
    SystemPermissions.query(Permission::ScreenRecording) == PermissionStatus::Granted
}
pub(crate) fn foreground() -> Result<Target, String> {
    #[cfg(target_os = "macos")]
    {
        super::screen_context_macos::foreground()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("当前平台不支持本机窗口分享。".into())
    }
}
pub(crate) fn capture(ticket: &Ticket) -> Result<Vec<u8>, String> {
    #[cfg(target_os = "macos")]
    {
        let result = super::screen_context_macos::capture(ticket);
        if result.is_err() {
            ticket.live.store(false, Ordering::SeqCst);
        }
        result
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = ticket;
        Err("当前平台不支持本机窗口分享。".into())
    }
}
#[cfg(any(target_os = "macos", test))]
pub(crate) fn encode(width: u32, height: u32, rgba: Vec<u8>) -> Result<Vec<u8>, String> {
    use vw_gateway_client::vw_api_types::voice::{MAX_SCREEN_BYTES, MAX_SCREEN_EDGE};
    if width == 0 || height == 0 || width > MAX_SCREEN_EDGE || height > MAX_SCREEN_EDGE {
        return Err("屏幕分辨率超过限制。".into());
    }
    let mut image = image::RgbaImage::from_raw(width, height, rgba)
        .map(image::DynamicImage::ImageRgba8)
        .ok_or("屏幕像素数据无效。")?;
    loop {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).map_err(|_| "无法编码屏幕帧。")?;
        if bytes.get_ref().len() <= MAX_SCREEN_BYTES {
            return Ok(bytes.into_inner());
        }
        let edge = image.width().max(image.height());
        if edge <= 64 {
            return Err("屏幕内容无法在安全大小限制内分享。".into());
        }
        image = image.resize(
            (image.width() / 2).max(1),
            (image.height() / 2).max(1),
            image::imageops::FilterType::Triangle,
        );
    }
}
impl Ticket {
    pub fn is_live(&self) -> bool {
        self.live.load(Ordering::SeqCst)
    }
}
