use super::{Error, read_text};
use encoding_rs::{BIG5, GBK, SHIFT_JIS, WINDOWS_1252};

#[test]
fn read_text_decodes_common_legacy_encodings_without_losing_whitespace() {
    let root = tempfile::tempdir().unwrap();
    for (encoding, original) in [
        (GBK, "  简体中文文件内容，这是一个编码检测测试。\r\n"),
        (BIG5, "  繁體中文檔案內容，這是一個編碼偵測測試。\r\n"),
        (SHIFT_JIS, "  これは日本語の文章です。文字コードを確認します。\n"),
        (WINDOWS_1252, "  Café déjà vu — voilà une très belle journée.\n"),
    ] {
        let (bytes, _, errors) = encoding.encode(original);
        assert!(!errors);
        std::fs::write(root.path().join("text.txt"), bytes).unwrap();
        assert_eq!(read_text(root.path(), "text.txt").unwrap(), original, "{}", encoding.name());
    }
}

#[test]
fn read_text_decodes_unicode_boms() {
    let root = tempfile::tempdir().unwrap();
    let original = "  中文 日本語 café 😀\r\n";
    let mut cases = vec![
        original.as_bytes().to_vec(),
        [b"\xef\xbb\xbf".as_slice(), original.as_bytes()].concat(),
    ];
    for little in [true, false] {
        let mut utf16 = if little { vec![0xff, 0xfe] } else { vec![0xfe, 0xff] };
        for unit in original.encode_utf16() {
            utf16.extend(if little { unit.to_le_bytes() } else { unit.to_be_bytes() });
        }
        cases.push(utf16);
        let mut utf32 = if little { vec![0xff, 0xfe, 0, 0] } else { vec![0, 0, 0xfe, 0xff] };
        for c in original.chars() {
            utf32.extend(if little { (c as u32).to_le_bytes() } else { (c as u32).to_be_bytes() });
        }
        cases.push(utf32);
    }
    cases.push(Vec::new());
    for bytes in cases {
        std::fs::write(root.path().join("text.txt"), &bytes).unwrap();
        assert_eq!(
            read_text(root.path(), "text.txt").unwrap(),
            if bytes.is_empty() { "" } else { original }
        );
    }
}

#[test]
fn read_text_rejects_binary_and_malformed_unicode_with_file_context() {
    let root = tempfile::tempdir().unwrap();
    for bytes in
        [b"hello\0world".as_slice(), b"%PDF-1.7", b"PK\x03\x04", b"\xff\xd8\xff", b"a\x01b"]
    {
        std::fs::write(root.path().join("unknown"), bytes).unwrap();
        let error = read_text(root.path(), "unknown").unwrap_err();
        assert!(matches!(error, Error::BinaryFile(_)));
        assert!(error.to_string().contains("unknown"));
    }
    for bytes in [
        b"\xff\xfe\x00".as_slice(),
        b"\xff\xfe\x00\xd8",
        b"\xef\xbb\xbf\xff",
        b"\x00\x00\xfe\xff\x00\x11\x00\x00",
    ] {
        std::fs::write(root.path().join("broken.txt"), bytes).unwrap();
        assert!(matches!(read_text(root.path(), "broken.txt"), Err(Error::InvalidText(_))));
    }
    std::fs::write(root.path().join("archive.zip"), b"looks like text").unwrap();
    assert!(matches!(read_text(root.path(), "archive.zip"), Err(Error::BinaryFile(_))));
}

#[test]
fn read_text_preserves_io_errors_and_rejects_workspace_escape() {
    let root = tempfile::tempdir().unwrap();
    assert!(matches!(read_text(root.path(), "missing.txt"), Err(Error::Io(_))));
    assert!(matches!(read_text(root.path(), "../outside"), Err(Error::AccessDenied(_))));
}

#[test]
fn read_text_accepts_scripts_and_text_source_extensions() {
    let root = tempfile::tempdir().unwrap();
    for ext in ["sh", "bash", "zsh", "fish", "bat", "cmd", "ps1", "wat", "ll", "s"] {
        let path = format!("source.{ext}");
        std::fs::write(root.path().join(&path), "echo hello\n").unwrap();
        assert_eq!(read_text(root.path(), &path).unwrap(), "echo hello\n");
    }
}
