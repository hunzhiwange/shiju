//! Shared, lossless text decoding for file endpoints.

use std::path::Path;

use chardetng::{EncodingDetector, Iso2022JpDetection, Utf8Detection};
use encoding_rs::Encoding;

use super::Error;
use super::pathing::{contains_path, is_binary_by_extension};

/// Read text inside a workspace, preserving whitespace and rejecting binary data.
/// Legacy encodings without a BOM are detected heuristically; malformed input is
/// never silently replaced with U+FFFD. Callers retain their own missing-file policy.
pub fn read_text(root: impl AsRef<Path>, file: &str) -> Result<String, Error> {
    let root = root.as_ref();
    let full = root.join(file);
    if !contains_path(root, &full) {
        return Err(Error::AccessDenied(file.to_string()));
    }
    let bytes = std::fs::read(full)?;
    if is_binary_by_extension(file) || has_binary_signature(&bytes) {
        return Err(Error::BinaryFile(file.to_string()));
    }
    decode(&bytes, file)
}

fn decode(bytes: &[u8], file: &str) -> Result<String, Error> {
    let invalid = || Error::InvalidText(file.to_string());
    let text = if bytes.starts_with(&[0xff, 0xfe, 0, 0]) || bytes.starts_with(&[0, 0, 0xfe, 0xff]) {
        let little_endian = bytes[0] == 0xff;
        let mut units = bytes[4..].chunks_exact(4);
        let text = units
            .by_ref()
            .map(|unit| {
                let unit = [unit[0], unit[1], unit[2], unit[3]];
                char::from_u32(if little_endian {
                    u32::from_le_bytes(unit)
                } else {
                    u32::from_be_bytes(unit)
                })
                .ok_or_else(invalid)
            })
            .collect::<Result<String, _>>()?;
        if !units.remainder().is_empty() {
            return Err(invalid());
        }
        text
    } else if let Some((encoding, bom_len)) = Encoding::for_bom(bytes) {
        encoding
            .decode_without_bom_handling_and_without_replacement(&bytes[bom_len..])
            .ok_or_else(invalid)?
            .into_owned()
    } else if let Ok(text) = std::str::from_utf8(bytes) {
        text.to_string()
    } else {
        // Exclude binary control bytes before statistical encoding detection.
        if bytes.iter().any(|&byte| byte < 0x80 && forbidden_control(char::from(byte))) {
            return Err(Error::BinaryFile(file.to_string()));
        }
        let mut detector = EncodingDetector::new(Iso2022JpDetection::Deny);
        detector.feed(bytes, true);
        let encoding = detector.guess(None, Utf8Detection::Deny);
        encoding
            .decode_without_bom_handling_and_without_replacement(bytes)
            .ok_or_else(invalid)?
            .into_owned()
    };
    if text.chars().any(forbidden_control) {
        return Err(Error::BinaryFile(file.to_string()));
    }
    Ok(text)
}

fn forbidden_control(c: char) -> bool {
    c.is_control() && !matches!(c, '\n' | '\r' | '\t' | '\u{c}')
}

fn has_binary_signature(bytes: &[u8]) -> bool {
    [
        b"\x89PNG\r\n\x1a\n".as_slice(),
        b"\xff\xd8\xff",
        b"GIF87a",
        b"GIF89a",
        b"%PDF-",
        b"PK\x03\x04",
        b"PK\x05\x06",
        b"\x1f\x8b",
        b"\x7fELF",
        b"SQLite format 3\0",
        b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1",
    ]
    .iter()
    .any(|signature| bytes.starts_with(signature))
}
