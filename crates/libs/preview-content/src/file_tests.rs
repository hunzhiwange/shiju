use super::*;
use std::io::Write;

fn write_temp_file(name: &str, bytes: &[u8]) -> tempfile::NamedTempFile {
    let mut file = tempfile::Builder::new().prefix(name).tempfile().expect("create temp file");
    file.write_all(bytes).expect("write temp file");
    file.as_file().sync_all().expect("sync temp file");
    file
}

#[test]
fn read_preview_sample_is_bounded_and_missing_files_are_treated_as_text() {
    let bytes = vec![b'a'; 5000];
    let file = write_temp_file("preview_sample", &bytes);
    let path = file.path().to_string_lossy().to_string();
    let missing = format!("{}.missing", file.path().display());

    assert_eq!(
        read_preview_sample(&path).map(|sample| sample.len()),
        Some(SAMPLE_BYTES + UTF8_LOOKAHEAD_BYTES),
    );
    assert!(file_looks_like_text(&missing));
}

#[test]
fn preview_reads_utf8_characters_crossing_the_sample_boundary() {
    for character in ["回", "🙂"] {
        let mut bytes = vec![b'a'; SAMPLE_BYTES - 1];
        bytes.extend_from_slice(character.as_bytes());
        let file = write_temp_file("utf8_sample", &bytes);
        let path = file.path().to_str().expect("UTF-8 path");

        let FilePreview::Text { content, truncated: false } = read_file_preview(path) else {
            panic!("expected a complete text preview");
        };
        assert!(content.contains(character));
    }
}

#[test]
fn preview_rejects_an_incomplete_or_invalid_utf8_character_at_the_sample_boundary() {
    let mut bytes = vec![b'a'; SAMPLE_BYTES - 1];
    bytes.push(0xe5);
    let incomplete = write_temp_file("incomplete_sample", &bytes);
    bytes.push(b'a');
    let invalid = write_temp_file("invalid_sample", &bytes);

    for file in [incomplete, invalid] {
        let path = file.path().to_str().expect("UTF-8 path");
        assert!(matches!(read_file_preview(path), FilePreview::Binary));
    }
}

#[test]
fn common_text_extensions_are_previewed_even_with_invalid_utf8() {
    let directory = tempfile::tempdir().expect("create temp directory");
    for name in ["notes.MD", "settings.toml", "main.rs"] {
        let path = directory.path().join(name);
        std::fs::write(&path, b"text\xff").expect("write temp file");
        let path = path.to_str().expect("UTF-8 path");

        assert!(file_looks_like_text(path));
        assert!(matches!(read_file_preview(path), FilePreview::Text { .. }));
    }
}

#[test]
fn unsupported_preview_kind_is_case_insensitive_and_rejects_archives_and_pdf() {
    for extension in ["zip", "TAR", "gz", "rar", "7z", "bz2", "xz"] {
        let path = format!("artifact.{extension}");
        assert_eq!(unsupported_preview_kind(&path), Some(UnsupportedPreviewKind::Archive));
    }
    assert_eq!(unsupported_preview_kind("document.PDF"), Some(UnsupportedPreviewKind::Pdf));
    assert_eq!(unsupported_preview_kind("notes.txt"), None);
    assert_eq!(unsupported_preview_kind("README"), None);
}
