//! 验证项目指令文件的逐级发现、优先级与目录边界。

use std::path::{Path, PathBuf};

use super::*;

fn write(path: impl AsRef<Path>, content: &str) -> PathBuf {
    let path = path.as_ref().to_path_buf();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create instruction parent");
    }
    std::fs::write(&path, content).expect("write instruction");
    path
}

fn discovered_paths(roots: DiscoveryRoots) -> Vec<PathBuf> {
    discover(&roots).into_iter().map(|source| source.filepath).collect()
}

#[test]
fn agents_discovery_orders_managed_user_project_and_local_root_to_cwd() {
    let fixture = tempfile::tempdir().expect("fixture directory");
    let managed = fixture.path().join("managed");
    let user = fixture.path().join("user");
    let project = fixture.path().join("project");
    let nested = project.join("nested");

    let expected = vec![
        write(managed.join("AGENTS.md"), "managed"),
        write(user.join("AGENTS.md"), "user"),
        write(project.join("AGENTS.md"), "project"),
        write(project.join(".vibewindow/AGENTS.md"), "project config"),
        write(project.join(".vibewindow/rules/a.md"), "project rule a"),
        write(project.join(".vibewindow/rules/b.md"), "project rule b"),
        write(project.join("AGENTS.local.md"), "project local"),
        write(nested.join("AGENTS.md"), "nested"),
        write(nested.join(".vibewindow/AGENTS.md"), "nested config"),
        write(nested.join(".vibewindow/rules/a.md"), "nested rule"),
        write(nested.join("AGENTS.local.md"), "nested local"),
    ];

    let actual = discovered_paths(DiscoveryRoots {
        managed_config: Some(managed),
        user_config: Some(user),
        current_directory: Some(nested),
        worktree: Some(project),
        project_config_enabled: true,
    });

    assert_eq!(actual, expected);
}

#[test]
fn agents_discovery_ignores_legacy_instruction_names() {
    let fixture = tempfile::tempdir().expect("fixture directory");
    let managed = fixture.path().join("managed");
    let user = fixture.path().join("user");
    let project = fixture.path().join("project");
    let nested = project.join("nested");

    let expected = vec![
        write(managed.join("AGENTS.md"), "managed"),
        write(user.join("AGENTS.md"), "user"),
        write(project.join("AGENTS.md"), "project"),
        write(nested.join("AGENTS.local.md"), "nested local"),
    ];
    for directory in [&managed, &user, &project, &nested] {
        write(directory.join("CLAUDE.md"), "legacy claude");
        write(directory.join("CONTEXT.md"), "legacy context");
    }

    let actual = discovered_paths(DiscoveryRoots {
        managed_config: Some(managed),
        user_config: Some(user),
        current_directory: Some(nested),
        worktree: Some(project),
        project_config_enabled: true,
    });

    assert_eq!(actual, expected);
}
