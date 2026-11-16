use super::normalize_str;
use std::path::PathBuf;

#[test]
fn normalize_str_keeps_paths_comparable() {
    let path = PathBuf::from("alpha").join("beta").join("AGENTS.md");

    assert_eq!(normalize_str(&path), path.to_string_lossy().to_string());
}
