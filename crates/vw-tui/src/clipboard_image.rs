//! 系统剪贴板图片到 TUI 内存附件的窄边界。

use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};

use crate::{V3Attachment, V3AttachmentKind};

pub(crate) fn is_image_paste_key(event: crossterm::event::KeyEvent) -> bool {
    event.kind == crossterm::event::KeyEventKind::Press
        && matches!(event.code, crossterm::event::KeyCode::Char(value) if value.eq_ignore_ascii_case(&'v'))
        && event.modifiers.intersects(
            crossterm::event::KeyModifiers::CONTROL | crossterm::event::KeyModifiers::ALT,
        )
}

pub(crate) fn read_clipboard_image_attachment() -> Result<V3Attachment, String> {
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
    let expected_len = usize::try_from(width)
        .ok()
        .and_then(|width| usize::try_from(height).ok().and_then(|height| width.checked_mul(height)))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "剪贴板图片尺寸超出范围".to_string())?;
    if rgba.len() != expected_len {
        return Err("剪贴板图片 RGBA 数据长度无效".to_string());
    }

    let mut encoded = Vec::new();
    PngEncoder::new(&mut encoded)
        .write_image(rgba, width, height, ColorType::Rgba8.into())
        .map_err(|error| format!("编码剪贴板图片失败：{error}"))?;
    Ok(encoded)
}

#[cfg(test)]
#[path = "clipboard_image_tests.rs"]
mod tests;
