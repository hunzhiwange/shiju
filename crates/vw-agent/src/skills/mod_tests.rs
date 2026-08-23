//! 确认新版技能模块的公开装配与兼容入口保持稳定。
//! 模块级测试覆盖旧目录兼容与后台同步去重，避免请求线程直接运行版本控制命令。

use super::*;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
struct PathGuard(Option<std::ffi::OsString>);

#[cfg(unix)]
impl Drop for PathGuard {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            unsafe { std::env::set_var("PATH", path) };
        } else {
            unsafe { std::env::remove_var("PATH") };
        }
    }
}

#[cfg(unix)]
fn prepend_path(path: &Path) -> PathGuard {
    let original = std::env::var_os("PATH");
    let mut paths = vec![path.to_path_buf()];
    paths.extend(std::env::split_paths(original.as_deref().unwrap_or_default()));
    unsafe { std::env::set_var("PATH", std::env::join_paths(paths).unwrap()) };
    PathGuard(original)
}

#[cfg(unix)]
fn write_fake_git(bin_dir: &Path, script: &str) {
    std::fs::create_dir_all(bin_dir).unwrap();
    let git = bin_dir.join("git");
    std::fs::write(&git, script).unwrap();
    let mut permissions = std::fs::metadata(&git).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(git, permissions).unwrap();
}

#[cfg(unix)]
fn wait_for_path(path: &Path) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !path.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(path.exists(), "timed out waiting for {}", path.display());
    std::thread::sleep(std::time::Duration::from_millis(50));
}

#[test]
fn discover_default_sources_includes_legacy_workspace_directory() {
    let workspace = tempfile::tempdir().expect("temp dir");
    let legacy = workspace.path().join("skills");
    std::fs::create_dir_all(&legacy).unwrap();
    let legacy = legacy.canonicalize().expect("legacy skills dir should canonicalize");

    let sources = discover_local_skill_source_dirs(Some(workspace.path()));

    assert!(
        sources.iter().any(|source| {
            source.kind == LocalSkillSourceKind::Workspace && source.path == legacy
        })
    );
}

#[cfg(unix)]
#[test]
// 请求线程只读取当前可用技能，仓库同步必须调度到后台以避免阻塞会话。
fn skill_loading_never_runs_open_skills_vcs_on_request_thread() {
    let _env_lock =
        crate::test_support::global_env_lock().lock().unwrap_or_else(|error| error.into_inner());
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let repo = temp.path().join("open-skills");
    let skill_dir = repo.join("skills").join("local-copy");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), "# Local copy\nAvailable immediately.\n").unwrap();
    let marker = repo.join(".vibewindow-open-skills-sync");
    std::fs::write(&marker, b"stale").unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&marker)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(std::time::SystemTime::UNIX_EPOCH))
        .unwrap();

    let bin = temp.path().join("bin");
    let vcs_called = temp.path().join("vcs-called");
    write_fake_git(
        &bin,
        &format!("#!/bin/sh\nsleep 1\nprintf called > '{}'\nexit 17\n", vcs_called.display()),
    );
    let _path = prepend_path(&bin);

    let started = std::time::Instant::now();
    let skills = load_skills_with_open_skills_config(
        &workspace,
        Some(true),
        Some(repo.to_str().unwrap()),
        SkillLoadMode::Full,
    );

    assert!(started.elapsed() < std::time::Duration::from_millis(500));
    assert!(skills.iter().any(|skill| skill.name == "local-copy"));
    assert!(!vcs_called.exists());
    wait_for_path(&vcs_called);
}

#[cfg(unix)]
#[test]
fn open_skills_repo_schedules_single_background_sync() {
    let _env_lock =
        crate::test_support::global_env_lock().lock().unwrap_or_else(|error| error.into_inner());
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let repo = temp.path().join("open-skills");
    let bin = temp.path().join("bin");
    let vcs_log = temp.path().join("vcs.log");
    write_fake_git(
        &bin,
        &format!(
            "#!/bin/sh\nprintf 'called\\n' >> '{}'\nsleep 1\nlast=''\nfor arg in \"$@\"; do last=\"$arg\"; done\nmkdir -p \"$last/skills/downloaded\"\nprintf '# Downloaded\\nReady next round.\\n' > \"$last/skills/downloaded/SKILL.md\"\nexit 0\n",
            vcs_log.display()
        ),
    );
    let _path = prepend_path(&bin);

    let started = std::time::Instant::now();
    let first = load_skills_with_open_skills_config(
        &workspace,
        Some(true),
        Some(repo.to_str().unwrap()),
        SkillLoadMode::Full,
    );
    let second = load_skills_with_open_skills_config(
        &workspace,
        Some(true),
        Some(repo.to_str().unwrap()),
        SkillLoadMode::Full,
    );

    assert!(started.elapsed() < std::time::Duration::from_millis(500));
    assert!(first.iter().all(|skill| skill.name != "downloaded"));
    assert!(second.iter().all(|skill| skill.name != "downloaded"));
    wait_for_path(&vcs_log);
    assert_eq!(std::fs::read_to_string(&vcs_log).unwrap().lines().count(), 1);
    wait_for_path(&repo.join(".vibewindow-open-skills-sync"));

    let next_round = load_skills_with_open_skills_config(
        &workspace,
        Some(true),
        Some(repo.to_str().unwrap()),
        SkillLoadMode::Full,
    );
    assert!(next_round.iter().any(|skill| skill.name == "downloaded"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn handle_command_forwards_remove_validation_errors() {
    let workspace = tempfile::tempdir().expect("temp dir");
    let mut config = crate::app::agent::config::Config::default();
    config.workspace_dir = workspace.path().to_path_buf();

    let error = handle_command(
        crate::app::agent::skill::SkillCommands::Remove { name: "../escape".into() },
        &config,
    )
    .unwrap_err();

    assert!(error.to_string().contains("Invalid skill name"));
}
