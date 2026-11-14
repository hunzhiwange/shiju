//! Bounded bitmap reads on the execution host. The caller must use an ImageRef
//! returned by this session's parsed transcript, never a client-supplied path.
use std::{fs::File, io::Read, path::PathBuf};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use vw_api_types::external_session::{ContentRef, ImageRef, ImageSource};
use super::{content_file, content_sqlite, source::ValidatedSource};

pub const MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;
const TOO_LARGE: &str = "图片超过20MiB";
const INVALID: &str = "图片数据格式无效";
const OUTSIDE: &str = "图片不属于当前会话授权目录";

/// No networking is performed. Local paths are checked against the authorized
/// source root on every read, including symlink rejection inherited from safe_join.
pub fn load_image(source: &ValidatedSource, image: &ImageRef) -> Result<Vec<u8>, String> {
    let expected = bitmap_mime(&image.media_type).ok_or("不支持的图片格式（包括SVG）")?;
    if u64::from(image.size) > MAX_IMAGE_BYTES { return Err(TOO_LARGE.into()); }
    let bytes = match &image.source {
        ImageSource::Inline { content } => {
            let encoded = match content {
                ContentRef::Sqlite { .. } => content_sqlite::resolve_content_ref(source, content)?,
                _ => content_file::resolve_content_ref(source, content)?,
            };
            decode_inline(&encoded, expected)?
        }
        ImageSource::LocalFile { path } => read_local(source, path, expected)?,
    };
    // Do not trust an extension or a claimed MIME type: SVG renamed to .png
    // must never reach the desktop renderer.
    if sniff_bitmap(&bytes) != Some(expected) { return Err(INVALID.into()); }
    Ok(bytes)
}

fn bitmap_mime(mime: &str) -> Option<&'static str> {
    match mime {
        "image/png" => Some("image/png"),
        "image/jpeg" | "image/jpg" => Some("image/jpeg"),
        "image/gif" => Some("image/gif"),
        "image/webp" => Some("image/webp"),
        "image/bmp" | "image/x-ms-bmp" => Some("image/bmp"),
        _ => None,
    }
}

fn decode_inline(encoded: &str, expected: &str) -> Result<Vec<u8>, String> {
    let data = if let Some(rest) = encoded.strip_prefix("data:") {
        let (header, data) = rest.split_once(',').ok_or(INVALID)?;
        let mime = header.strip_suffix(";base64").ok_or(INVALID)?;
        if bitmap_mime(mime) != Some(expected) { return Err(INVALID.into()); }
        data
    } else { encoded };
    // Bound allocation before whitespace removal or base64 decoding.
    let max_encoded = MAX_IMAGE_BYTES.div_ceil(3) * 4;
    if data.len() as u64 > max_encoded + 8 { return Err(TOO_LARGE.into()); }
    let compact: String = data.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    let bytes = STANDARD.decode(compact).map_err(|_| INVALID)?;
    if bytes.len() as u64 > MAX_IMAGE_BYTES { return Err(TOO_LARGE.into()); }
    Ok(bytes)
}

fn read_local(source: &ValidatedSource, raw: &str, expected: &str) -> Result<Vec<u8>, String> {
    let path = if raw.starts_with("file:") {
        let url = reqwest::Url::parse(raw).map_err(|_| OUTSIDE)?;
        if url.scheme() != "file" || url.host_str().is_some_and(|host| !host.is_empty() && host != "localhost")
            || url.query().is_some() || url.fragment().is_some() {
            return Err(OUTSIDE.into());
        }
        url.to_file_path().map_err(|_| OUTSIDE)?
    } else { PathBuf::from(raw) };
    if !path.is_absolute() { return Err(OUTSIDE.into()); }
    let relative = path.strip_prefix(source.root()).map_err(|_| OUTSIDE)?;
    let path = content_file::safe_join(source.root(), relative.to_str().ok_or(OUTSIDE)?)?;
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_ascii_lowercase();
    let mime = match extension.as_str() {
        "png" => "image/png", "jpg" | "jpeg" => "image/jpeg", "gif" => "image/gif",
        "webp" => "image/webp", "bmp" => "image/bmp", _ => return Err("不支持的图片格式（包括SVG）".into()),
    };
    if mime != expected { return Err(INVALID.into()); }
    let file = File::open(path).map_err(|_| "无法读取图片")?;
    let metadata = file.metadata().map_err(|_| "无法读取图片")?;
    if !metadata.is_file() { return Err(OUTSIDE.into()); }
    if metadata.len() > MAX_IMAGE_BYTES { return Err(TOO_LARGE.into()); }
    let mut bytes = Vec::new();
    file.take(MAX_IMAGE_BYTES + 1).read_to_end(&mut bytes).map_err(|_| "无法读取图片")?;
    if bytes.len() as u64 > MAX_IMAGE_BYTES { return Err(TOO_LARGE.into()); }
    Ok(bytes)
}

fn sniff_bitmap(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") { Some("image/png") }
    else if bytes.starts_with(b"\xff\xd8\xff") { Some("image/jpeg") }
    else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") { Some("image/gif") }
    else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" { Some("image/webp") }
    else if bytes.starts_with(b"BM") { Some("image/bmp") }
    else { None }
}
