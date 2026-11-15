use super::{inventory::*, local_install::LocalSkill};
use std::fs;

#[test]
fn cp12_inventory_success() {
    let root = tempfile::tempdir().unwrap();
    let tree = root.path().join("sample");
    fs::create_dir(&tree).unwrap();
    fs::write(tree.join("SKILL.md"), "---\nname: Sample\nversion: '1.2'\n---\nInstructions").unwrap();
    fs::write(tree.join("z.txt"), "last").unwrap();
    fs::write(tree.join("a.txt"), "first").unwrap();
    let saved = LocalSkill { directory: "sample".into(), name: "old".into(), description: None, source: tree.clone() };
    let detail = get_all_installed(root.path(), &[saved.clone()]).unwrap().remove(0);
    assert!(detail.present);
    assert_eq!(detail.skill.name, "Sample");
    assert_eq!(detail.version.as_deref(), Some("1.2"));
    assert_eq!(detail.resources, vec![std::path::PathBuf::from("a.txt"), "z.txt".into()]);
    let hash = detail.content_hash.unwrap();
    fs::remove_file(tree.join("a.txt")).unwrap();
    fs::write(tree.join("a.txt"), "first").unwrap();
    assert_eq!(compute_dir_hash(&tree).unwrap(), hash);
    fs::write(tree.join("a.txt"), "changed").unwrap();
    assert_ne!(compute_dir_hash(&tree).unwrap(), hash);
    fs::remove_dir_all(&tree).unwrap();
    assert!(local_hash_for_update_check(root.path(), "sample").unwrap().is_none());
    let detail = get_all_installed(root.path(), &[saved]).unwrap().remove(0);
    assert!(!detail.present);
    assert!(detail.content_hash.is_none());
}

#[test]
fn cp12_inventory_rejects_without_side_effects() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    fs::write(external.path().join("private"), "unchanged").unwrap();
    assert!(local_hash_for_update_check(root.path(), "../escape").is_err());
    std::os::unix::fs::symlink(external.path(), root.path().join("linked")).unwrap();
    assert!(local_hash_for_update_check(root.path(), "linked").is_err());
    let tree = root.path().join("safe");
    fs::create_dir(&tree).unwrap();
    std::os::unix::fs::symlink(external.path().join("private"), tree.join("resource")).unwrap();
    assert!(compute_dir_hash(&tree).is_err());
    assert_eq!(fs::read_to_string(external.path().join("private")).unwrap(), "unchanged");
}

#[test]
fn cp12_inventory_supports_manifest_without_markdown() {
    let root = tempfile::tempdir().unwrap();
    let tree = root.path().join("sample");
    fs::create_dir(&tree).unwrap();
    fs::write(tree.join("SKILL.toml"), "name = 'Manifest'\ndescription = 'Details'\n").unwrap();
    let saved = LocalSkill { directory: "sample".into(), name: "old".into(), description: None, source: tree };
    let details = get_all_installed(root.path(), &[saved.clone()]).unwrap();
    assert_eq!(details[0].skill.name, "Manifest");
    assert!(details[0].version.is_none());
    assert!(details[0].manifest.is_some());
    assert!(get_all_installed(root.path(), &[saved.clone(), saved]).is_err());
}
