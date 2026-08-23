//! 验证文件系统路径边界、规范化及向上查找辅助函数。

use super::filesystem::{contains, exists, find_up, glob_up, is_dir, normalize_path, overlaps, up};

#[tokio::test]
async fn path_helpers_detect_boundaries_and_walk_upwards() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    let child = root.join("a").join("b");
    std::fs::create_dir_all(&child).expect("mkdir");
    std::fs::write(root.join("Cargo.toml"), "x").expect("write");
    std::fs::write(child.join("local.txt"), "x").expect("write");

    assert!(exists(root.join("Cargo.toml")));
    assert!(is_dir(&child));
    assert_eq!(normalize_path(&child), child);
    assert!(contains(root, &child));
    assert!(overlaps(root, &child));
    assert_eq!(find_up("Cargo.toml", &child, None::<&std::path::Path>).await.len(), 1);
    assert_eq!(up(&["Cargo.toml"], &child, None::<&std::path::Path>).len(), 1);
    assert_eq!(glob_up("*.txt", &child, Some(root)).len(), 1);
}

#[test]
fn path_helpers_reject_missing_paths_and_fall_back_to_lexical_containment() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing_parent = dir.path().join("missing");
    let missing_child = missing_parent.join("child");

    assert!(!exists(&missing_parent));
    assert!(!is_dir(&missing_parent));
    assert!(contains(&missing_parent, &missing_child));
    assert!(overlaps(&missing_child, &missing_parent));
    assert!(!overlaps(&missing_parent, dir.path().join("sibling")));
}

#[tokio::test]
async fn upward_walkers_include_matches_at_stop_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    let stop = dir.path().join("stop");
    let child = stop.join("child");
    std::fs::create_dir_all(&child).expect("mkdir");
    let marker = stop.join("marker.txt");
    std::fs::write(&marker, "marker").expect("write marker");

    assert_eq!(find_up("marker.txt", &child, Some(&stop)).await, vec![marker.clone()]);
    assert_eq!(up(&["marker.txt"], &child, Some(&stop)), vec![marker.clone()]);
    assert_eq!(glob_up("marker.*", &child, Some(&stop)), vec![marker]);
}
