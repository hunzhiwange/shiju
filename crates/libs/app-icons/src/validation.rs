//! Bounded, complete PNG validation without storage or runtime dependencies.
use base64::Engine;
use std::io::Cursor;

/// Accepted custom icon data URI prefix.
pub const CUSTOM_ICON_PREFIX: &str = "data:image/png;base64,";
/// Maximum length of the entire data URI in bytes.
pub const MAX_CUSTOM_ICON_LENGTH: usize = 64 * 1024;
const MAX_CUSTOM_ICON_DIMENSION: u32 = 256;
// Twice the memory needed for a 256 × 256 16-bit RGBA image.
const DECODE_LIMIT_BYTES: usize = 1024 * 1024;

/// 完整解码一遍：签名、块校验和、像素数据都要完好，宽高在上限内，解码内存有上限。
pub fn is_valid_custom_icon(icon: &str) -> bool {
    if icon.len() > MAX_CUSTOM_ICON_LENGTH {
        return false;
    }
    let Some(encoded) = icon.strip_prefix(CUSTOM_ICON_PREFIX) else {
        return false;
    };
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded) else {
        return false;
    };
    let decoder = png::Decoder::new_with_limits(
        Cursor::new(bytes.as_slice()),
        png::Limits { bytes: DECODE_LIMIT_BYTES },
    );
    let Ok(mut reader) = decoder.read_info() else {
        return false;
    };
    let (width, height) = reader.info().size();
    // 先看宽高再分配像素缓冲区
    if width == 0
        || height == 0
        || width > MAX_CUSTOM_ICON_DIMENSION
        || height > MAX_CUSTOM_ICON_DIMENSION
    {
        return false;
    }
    let Some(size) = reader.output_buffer_size() else {
        return false;
    };
    let mut buf = vec![0; size];
    reader.next_frame(&mut buf).is_ok() && reader.finish().is_ok()
}
