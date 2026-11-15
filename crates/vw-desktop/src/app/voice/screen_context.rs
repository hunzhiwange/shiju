//! Desktop window capture and encoding.
pub use desktop_voice::screen_context::{State, Target, Ticket};
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
            ticket.cancel();
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
    use vw_api_voice::{MAX_SCREEN_BYTES, MAX_SCREEN_EDGE};
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
