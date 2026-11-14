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

    assert_eq!(read_preview_sample(&path).map(|sample| sample.len()), Some(4096));
    assert!(file_looks_like_text(&missing));
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
