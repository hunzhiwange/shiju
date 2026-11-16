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

/// Recognize a single existing image path without evaluating shell syntax.
/// Reading the file and enforcing workspace permissions remain request-boundary work.
pub fn image_path_attachment(pasted: &str) -> Option<V3Attachment> {
    let path = normalize_pasted_path(pasted)?;
    if !path.is_file() {
        return None;
    }
    let media_type = match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        _ => return None,
    };
    Some(V3Attachment {
        kind: V3AttachmentKind::Image,
        label: path.file_name()?.to_str()?.to_owned(),
        path: Some(path.to_str()?.to_owned()),
        media_type: Some(media_type.to_owned()),
        text: None,
        bytes: None,
    })
}

fn normalize_pasted_path(pasted: &str) -> Option<std::path::PathBuf> {
    // Reject before trimming: even trailing newlines belong to text pastes.
    if pasted.chars().any(char::is_control) {
        return None;
    }
    let pasted = pasted.trim();
    let quoted = pasted.starts_with(['\'', '"']);
    let value = if quoted {
        let quote = pasted.chars().next()?;
        pasted.strip_prefix(quote)?.strip_suffix(quote)?
    } else {
        pasted
    };
    if let Some(uri) = value.strip_prefix("file://") {
        let local = if uri.starts_with('/') {
            uri
        } else {
            uri.strip_prefix("localhost").filter(|rest| rest.starts_with('/'))?
        };
        if local.contains(['?', '#']) {
            return None;
        }
        let mut decoded = Vec::new();
        let mut bytes = local.bytes();
        while let Some(byte) = bytes.next() {
            if byte == b'%' {
                let high = char::from(bytes.next()?).to_digit(16)?;
                let low = char::from(bytes.next()?).to_digit(16)?;
                decoded.push((high * 16 + low) as u8);
            } else {
                decoded.push(byte);
            }
        }
        let decoded = String::from_utf8(decoded).ok()?;
        if decoded.chars().any(char::is_control) {
            return None;
        }
        #[cfg(windows)]
        let decoded = decoded.strip_prefix('/').unwrap_or(&decoded).to_owned();
        return Some(decoded.into());
    }
    if value.is_empty() {
        return None;
    }
    // Windows separators are literal; POSIX escaping only unescapes a following
    // space, quote or backslash. No variables, substitutions or expansion run.
    let windows = cfg!(windows);
    let mut normalized = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if !quoted && !windows && ch == '\\' {
            let escaped = chars.next()?;
            if !matches!(escaped, ' ' | '\\' | '\'' | '"') {
                return None;
            }
            normalized.push(escaped);
        } else if !quoted
            && (ch.is_whitespace()
                || matches!(ch, '\'' | '"' | ';' | '|' | '&' | '$' | '`' | '<' | '>'))
        {
            return None;
        } else {
            normalized.push(ch);
        }
    }
    Some(normalized.into())
}

#[cfg(test)]
#[path = "terminal_req_06_tests.rs"]
mod terminal_req_06_tests;
