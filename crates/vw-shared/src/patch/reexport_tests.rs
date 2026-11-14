use super::{apply_hunks_to_files, parse_patch};

#[test]
fn patch_reexport_preserves_types_and_file_application() {
    let parsed =
        parse_patch("*** Begin Patch\n*** Add File: nested/example.txt\n+hello\n*** End Patch")
            .expect("old entry should parse the patch");
    let hunks: Vec<::patch::patch::Hunk> = parsed.hunks;
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");

    let affected = apply_hunks_to_files(&hunks, Some(temp_dir.path()))
        .expect("old entry should apply the patch");
    let path = temp_dir.path().join("nested/example.txt");

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    assert_eq!(affected.added, vec![path.to_string_lossy().to_string()]);
    assert!(affected.modified.is_empty());
    assert!(affected.deleted.is_empty());
}
