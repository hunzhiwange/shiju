use super::patch_dotenv::DotenvPatch;

#[test]
fn cc04_patch_dotenv_keeps_comments_and_unrelated_keys() {
    let patch = DotenvPatch {
        set: vec![("GEMINI_API_KEY".into(), "\"new # key\"".into())],
        remove: vec!["OLD".into()],
        ..Default::default()
    };
    let pre = "# 中文 🚀\r\nOTHER='literal $X'\nexport GEMINI_API_KEY=old\r\nOLD=1\r\nGEMINI_API_KEY=duplicate\nunknown line";
    assert_eq!(
        String::from_utf8(patch.apply(Some(pre.as_bytes())).unwrap()).unwrap(),
        "# 中文 🚀\r\nOTHER='literal $X'\nexport GEMINI_API_KEY=\"new # key\"\r\nunknown line"
    );
}

#[test]
fn cc04_patch_dotenv_rejects_invalid_input() {
    for (key, value) in
        [("BAD=KEY", "x"), ("", "x"), ("X", "a\nINJECT=1"), ("X", "a\rINJECT=1"), ("X", "\0")]
    {
        let patch = DotenvPatch { set: vec![(key.into(), value.into())], ..Default::default() };
        assert!(patch.apply(Some(b"SAFE=1\n")).is_err(), "invalid input accepted");
    }
    assert!(DotenvPatch::default().apply(Some(&[0xff])).is_err());
}

#[test]
fn cc04_patch_dotenv_appends_removes_and_reads_raw_quotes() {
    use super::patch_dotenv::entries;
    let patch = DotenvPatch {
        clear: Some(|key| key.starts_with("GEMINI_")),
        set: vec![("GEMINI_MODEL".into(), "m".into())],
        remove: vec!["GEMINI_MODEL".into()],
        remove_if: vec![("X".into(), vec!["1".into()])],
    };
    for (pre, expected) in [
        (None, "GEMINI_MODEL=m\n"),
        (Some("DEBUG=1"), "DEBUG=1\nGEMINI_MODEL=m"),
        (Some("DEBUG=1\r\n"), "DEBUG=1\r\nGEMINI_MODEL=m\r\n"),
        (Some("X=\"1\"\nGEMINI_OLD=1\nexport X='1'\n"), "GEMINI_MODEL=m\n"),
        (Some("X=0\n"), "X=0\nGEMINI_MODEL=m\n"),
    ] {
        let out = patch.apply(pre.map(str::as_bytes)).unwrap();
        assert_eq!(out, expected.as_bytes());
        assert_eq!(patch.apply(Some(&out)).unwrap(), out);
    }
    assert_eq!(
        entries("# c\nX=1\nY = 2\nexport X=\"3\"\n"),
        vec![("X".into(), "\"3\"".into()), ("Y".into(), "2".into())]
    );
    let raw = b"# no change\r\ninvalid line\nX='a'";
    assert_eq!(DotenvPatch::default().apply(Some(raw)).unwrap(), raw);
}

#[test]
fn cc04_patch_dotenv_rejects_invalid_removals_and_size_without_file_changes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".env");
    std::fs::write(&path, b"SAFE=1\n").unwrap();
    for patch in [
        DotenvPatch { remove: vec!["X\nY".into()], ..Default::default() },
        DotenvPatch { remove_if: vec![("X".into(), vec!["a\nb".into()])], ..Default::default() },
        DotenvPatch { set: vec![("X".into(), "a".repeat(1024 * 1024))], ..Default::default() },
    ] {
        assert!(patch.apply(Some(&std::fs::read(&path).unwrap())).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"SAFE=1\n");
    }
    assert!(DotenvPatch::default().apply(Some(&vec![b'a'; 1024 * 1024 + 1])).is_err());
}
