//! 系统剪贴板图片到 TUI 内存附件的窄边界。

use std::io::Write;

use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};

use tui_contracts::event::{V3Attachment, V3AttachmentKind};

// Keep both decoded pixels and encoded attachments within the existing 5 MiB boundary.
const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

pub fn is_image_paste_key(event: crossterm::event::KeyEvent) -> bool {
    event.kind == crossterm::event::KeyEventKind::Press
        && matches!(event.code, crossterm::event::KeyCode::Char(value) if value.eq_ignore_ascii_case(&'v'))
        && matches!(
            event.modifiers,
            crossterm::event::KeyModifiers::CONTROL | crossterm::event::KeyModifiers::ALT
        )
}

pub fn read_clipboard_image_attachment() -> Result<V3Attachment, String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|error| format!("无法访问系统剪贴板：{error}"))?;
    let image = clipboard.get_image().map_err(|error| format!("剪贴板中没有可用图片：{error}"))?;
    let width = u32::try_from(image.width).map_err(|_| "剪贴板图片宽度超出范围".to_string())?;
    let height = u32::try_from(image.height).map_err(|_| "剪贴板图片高度超出范围".to_string())?;
    let bytes = encode_rgba_as_png(width, height, image.bytes.as_ref())?;

    Ok(V3Attachment {
        kind: V3AttachmentKind::Image,
        label: format!("clipboard-{width}x{height}.png"),
        path: None,
        media_type: Some("image/png".to_string()),
        text: None,
        bytes: Some(bytes),
    })
}

fn encode_rgba_as_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 {
        return Err("剪贴板图片尺寸不能为零".to_string());
    }
    let expected_len = usize::try_from(width)
        .ok()
        .and_then(|width| usize::try_from(height).ok().and_then(|height| width.checked_mul(height)))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "剪贴板图片尺寸超出范围".to_string())?;
    if expected_len > MAX_IMAGE_BYTES {
        return Err("剪贴板图片原始数据超过 5 MiB 限制".to_string());
    }
    if rgba.len() != expected_len {
        return Err("剪贴板图片 RGBA 数据长度无效".to_string());
    }

    let mut encoded = Vec::new();
    PngEncoder::new(BoundedPngWriter(&mut encoded))
        .write_image(rgba, width, height, ColorType::Rgba8.into())
        .map_err(|error| format!("编码剪贴板图片失败：{error}"))?;
    Ok(encoded)
}

#[cfg(test)]
#[path = "clipboard_image_tests.rs"]
mod tests;

// Enforce the output limit while encoding, before a PNG allocation can grow past it.
struct BoundedPngWriter<'a>(&'a mut Vec<u8>);

impl Write for BoundedPngWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_IMAGE_BYTES.saturating_sub(self.0.len()) {
            return Err(std::io::Error::other("剪贴板图片 PNG 超过 5 MiB 限制"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "terminal_req_03_tests.rs"]
mod terminal_req_03_tests;
