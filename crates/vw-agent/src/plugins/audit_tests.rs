use super::{canonical_plugin_root, plugin_manifest_path, resolve_component_path};
use std::fs;

#[test]
fn canonical_plugin_root_reports_missing_metadata() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing = dir.path().join("missing-plugin");

    let err = canonical_plugin_root(&missing).expect_err("missing root rejected").to_string();

    assert!(err.contains("failed to read plugin root metadata"), "{err}");
    assert!(err.contains("missing-plugin"), "{err}");
}

#[test]
fn canonical_plugin_root_rejects_regular_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = dir.path().join("plugin-file");
    fs::write(&file, "not a directory").expect("plugin file");

    let err = canonical_plugin_root(&file).expect_err("regular file rejected").to_string();

    assert!(err.contains("must be a directory"), "{err}");
    assert!(err.contains("plugin-file"), "{err}");
}

#[cfg(unix)]
#[test]
fn canonical_plugin_root_rejects_symlink_root() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().expect("temp dir");
    let real_root = dir.path().join("real-plugin");
    let linked_root = dir.path().join("linked-plugin");
    fs::create_dir_all(&real_root).expect("real root");
    symlink(&real_root, &linked_root).expect("plugin root symlink");

    let err =
        canonical_plugin_root(&linked_root).expect_err("symlink plugin root rejected").to_string();

    assert!(err.contains("must not be a symlink"), "{err}");
    assert!(err.contains("linked-plugin"), "{err}");
}

#[test]
fn plugin_manifest_path_rejects_missing_manifest() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugin_root = dir.path().join("plugin");
    fs::create_dir_all(&plugin_root).expect("plugin root");
    let canonical_root = canonical_plugin_root(&plugin_root).expect("canonical root");

    let err =
        plugin_manifest_path(&canonical_root).expect_err("missing manifest rejected").to_string();

    assert!(err.contains("plugin manifest not found"), "{err}");
    assert!(err.contains("plugin.json"), "{err}");
}

#[test]
fn plugin_manifest_path_accepts_manifest_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugin_root = dir.path().join("plugin");
    let manifest_dir = plugin_root.join(".codex-plugin");
    fs::create_dir_all(&manifest_dir).expect("manifest dir");
    fs::write(manifest_dir.join("plugin.json"), "{}").expect("plugin manifest");
    let canonical_root = canonical_plugin_root(&plugin_root).expect("canonical root");

    let manifest = plugin_manifest_path(&canonical_root).expect("manifest path");

    assert!(manifest.ends_with(".codex-plugin/plugin.json"));
    assert!(manifest.is_file());
}

#[test]
fn plugin_manifest_path_rejects_manifest_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugin_root = dir.path().join("plugin");
    fs::create_dir_all(plugin_root.join(".codex-plugin/plugin.json")).expect("manifest dir");
    let canonical_root = canonical_plugin_root(&plugin_root).expect("canonical root");

    let err =
        plugin_manifest_path(&canonical_root).expect_err("manifest directory rejected").to_string();

    assert!(err.contains("Plugin manifest must be a file"), "{err}");
    assert!(err.contains("plugin.json"), "{err}");
}

#[test]
fn plugin_manifest_path_rejects_noncanonical_root_escape() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugin_root = dir.path().join("plugin");
    let manifest_dir = plugin_root.join(".codex-plugin");
    fs::create_dir_all(&manifest_dir).expect("manifest dir");
    fs::write(manifest_dir.join("plugin.json"), "{}").expect("plugin manifest");
    let noncanonical_root = plugin_root.join("..").join("plugin");

    let err = plugin_manifest_path(&noncanonical_root)
        .expect_err("noncanonical root rejected")
        .to_string();

    assert!(err.contains("Plugin manifest path escapes plugin root"), "{err}");
    assert!(err.contains("plugin.json"), "{err}");
}

#[test]
fn resolve_component_path_requires_dot_slash_prefix() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugin_root = dir.path().join("plugin");
    fs::create_dir_all(plugin_root.join("skills")).expect("skills dir");
    let canonical_root = canonical_plugin_root(&plugin_root).expect("canonical root");

    let err = resolve_component_path(&canonical_root, "skills", "skills")
        .expect_err("missing dot slash rejected")
        .to_string();

    assert!(err.contains("field skills"), "{err}");
    assert!(err.contains("must start with './'"), "{err}");
}

#[test]
fn resolve_component_path_accepts_existing_relative_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugin_root = dir.path().join("plugin");
    let skill_path = plugin_root.join("skills/example/SKILL.md");
    fs::create_dir_all(skill_path.parent().expect("skill parent")).expect("skills dir");
    fs::write(&skill_path, "# Example").expect("skill file");
    let canonical_root = canonical_plugin_root(&plugin_root).expect("canonical root");

    let plugin_path =
        resolve_component_path(&canonical_root, "skills", "./skills/example/SKILL.md")
            .expect("component path");

    assert_eq!(plugin_path.raw, "./skills/example/SKILL.md");
    assert_eq!(plugin_path.canonical_path, skill_path.canonicalize().expect("canonical skill"));
}

#[test]
fn resolve_component_path_rejects_missing_existing_relative_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugin_root = dir.path().join("plugin");
    fs::create_dir_all(&plugin_root).expect("plugin root");
    let canonical_root = canonical_plugin_root(&plugin_root).expect("canonical root");

    let err = resolve_component_path(&canonical_root, "skills", "./skills/missing/SKILL.md")
        .expect_err("missing component rejected")
        .to_string();

    assert!(err.contains("field skills"), "{err}");
    assert!(err.contains("path does not exist"), "{err}");
}

#[test]
fn resolve_component_path_rejects_parent_escape_after_canonicalization() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugin_root = dir.path().join("plugin");
    let outside_path = dir.path().join("outside/SKILL.md");
    fs::create_dir_all(&plugin_root).expect("plugin root");
    fs::create_dir_all(outside_path.parent().expect("outside parent")).expect("outside dir");
    fs::write(&outside_path, "# Outside").expect("outside skill");
    let canonical_root = canonical_plugin_root(&plugin_root).expect("canonical root");

    let err = resolve_component_path(&canonical_root, "skills", "./../outside/SKILL.md")
        .expect_err("parent escape rejected")
        .to_string();

    assert!(err.contains("field skills"), "{err}");
    assert!(err.contains("escapes plugin root"), "{err}");
}

#[cfg(unix)]
#[test]
fn resolve_component_path_rejects_symlink_component() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().expect("temp dir");
    let plugin_root = dir.path().join("plugin");
    let outside = dir.path().join("outside");
    fs::create_dir_all(&plugin_root).expect("plugin root");
    fs::create_dir_all(&outside).expect("outside dir");
    symlink(&outside, plugin_root.join("linked-skills")).expect("skills symlink");
    let canonical_root = canonical_plugin_root(&plugin_root).expect("canonical root");

    let err = resolve_component_path(&canonical_root, "skills", "./linked-skills")
        .expect_err("symlink component rejected")
        .to_string();

    assert!(err.contains("field skills"), "{err}");
    assert!(err.contains("includes symlink"), "{err}");
}
