//! 验证旧版技能安装的来源校验、复制与冲突处理行为。
//! 安装夹具覆盖本地目录、Git 仓库、符号链接和失败清理，目标冲突时不允许半安装。

use super::*;
use std::ffi::OsString;

#[cfg(not(target_arch = "wasm32"))]
struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

#[cfg(not(target_arch = "wasm32"))]
impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.previous {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_clean_skill(dir: &Path, title: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("SKILL.md"), format!("# {title}\n\nSafe instructions.")).unwrap();
}

fn default_security() -> SkillSecurityConfig {
    SkillSecurityConfig::default()
}

#[cfg(not(target_arch = "wasm32"))]
// 本地 Git 仓库只作为安装来源夹具，安装完成后目标中不得残留版本控制元数据。
fn run_git(repo: &Path, args: &[&str]) {
    let output = git_std_command().args(args).current_dir(repo).output().expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn git_source_detection_accepts_explicit_git_forms() {
    assert!(is_git_source("https://github.com/acme/skill.git"));
    assert!(is_git_source("ssh://git@github.com/acme/skill.git"));
    assert!(is_git_source("git@github.com:acme/skill.git"));
    assert!(!is_git_source("https://skills.sh/acme/repo/skill"));
    assert!(!is_git_source("/local/path"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn detects_single_new_install_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = snapshot_skill_children(dir.path()).expect("snapshot");
    std::fs::create_dir(dir.path().join("new-skill")).expect("create skill dir");

    let detected = detect_newly_installed_directory(dir.path(), &before).expect("new dir");
    assert_eq!(detected.file_name().and_then(|name| name.to_str()), Some("new-skill"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn detects_missing_and_ambiguous_new_install_directories() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = snapshot_skill_children(dir.path()).expect("snapshot");
    let missing = detect_newly_installed_directory(dir.path(), &before).unwrap_err();
    assert!(missing.to_string().contains("no new directory"));

    std::fs::create_dir(dir.path().join("one")).unwrap();
    std::fs::create_dir(dir.path().join("two")).unwrap();
    let multiple = detect_newly_installed_directory(dir.path(), &before).unwrap_err();
    assert!(multiple.to_string().contains("multiple new directories"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn newly_installed_directory_detection_ignores_regular_files() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = snapshot_skill_children(dir.path()).expect("snapshot");
    std::fs::write(dir.path().join("clone.log"), "not an installed skill").unwrap();

    let err = detect_newly_installed_directory(dir.path(), &before).unwrap_err();

    assert!(err.to_string().contains("no new directory"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn snapshot_skill_children_rejects_missing_root() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing = dir.path().join("missing");

    assert!(snapshot_skill_children(&missing).is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn snapshot_skill_children_records_existing_files_and_directories() {
    let dir = tempfile::tempdir().expect("temp dir");
    let child_dir = dir.path().join("existing-skill");
    let child_file = dir.path().join("README.md");
    std::fs::create_dir(&child_dir).unwrap();
    std::fs::write(&child_file, "fixture").unwrap();

    let snapshot = snapshot_skill_children(dir.path()).unwrap();

    assert_eq!(snapshot.len(), 2);
    assert!(snapshot.contains(&child_dir));
    assert!(snapshot.contains(&child_file));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn installed_directory_detection_reports_missing_root() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing = dir.path().join("missing");

    let err = detect_newly_installed_directory(&missing, &HashSet::new()).unwrap_err();

    assert_eq!(
        err.downcast_ref::<std::io::Error>().map(std::io::Error::kind),
        Some(std::io::ErrorKind::NotFound)
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn git_source_detection_rejects_malformed_scheme_and_scp_forms() {
    assert!(!is_git_source("https:///owner/repo.git"));
    assert!(!is_git_source("https://github.com/owner/repo"));
    assert!(!is_git_source("git@github.com:"));
    assert!(!is_git_source("git/github.com:owner/repo.git"));
    assert!(!is_git_source("git@github.com/owner:repo.git"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn git_source_detection_covers_supported_protocol_boundaries() {
    assert!(is_git_source("http://example.com/owner/repo.git"));
    assert!(is_git_source("git://example.com/owner/repo"));
    assert!(is_git_source("user@example.com:owner/repo"));

    assert!(!is_git_source("https://example.com/owner/repo.git?ref=main"));
    assert!(!is_git_source("ssh:///owner/repo"));
    assert!(!is_git_source("user@:owner/repo"));
    assert!(!is_git_source("@example.com:owner/repo"));
    assert!(!is_git_source("user\\name@example.com:owner/repo"));
    assert!(!is_git_source("user@example.com\\host:owner/repo"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn init_skills_dir_writes_readme_policy_and_builtin_skills() {
    let dir = tempfile::tempdir().expect("temp dir");

    init_skills_dir(dir.path()).unwrap();

    let skills = skills_dir(dir.path());
    assert!(skills.join("README.md").is_file());
    assert!(skills.join(crate::app::agent::skill::constants::SKILL_DOWNLOAD_POLICY_FILE).is_file());
    for builtin in crate::app::agent::skill::constants::BUILTIN_PRELOADED_SKILLS {
        assert!(skills.join(builtin.dir_name).join("SKILL.md").is_file());
        assert!(skills.join(builtin.dir_name).join("_meta.json").is_file());
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn init_skills_dir_preserves_existing_readme_and_builtin_skill() {
    let dir = tempfile::tempdir().expect("temp dir");
    let skills = skills_dir(dir.path());
    let readme = skills.join("README.md");
    let builtin = &BUILTIN_PRELOADED_SKILLS[0];
    let builtin_dir = skills.join(builtin.dir_name);
    std::fs::create_dir_all(&builtin_dir).unwrap();
    std::fs::write(&readme, "custom readme").unwrap();
    std::fs::write(builtin_dir.join("SKILL.md"), "custom builtin").unwrap();

    init_skills_dir(dir.path()).unwrap();

    assert_eq!(std::fs::read_to_string(readme).unwrap(), "custom readme");
    assert_eq!(std::fs::read_to_string(builtin_dir.join("SKILL.md")).unwrap(), "custom builtin");
    assert!(!builtin_dir.join("_meta.json").exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn builtin_skill_metadata_records_preloaded_source() {
    let dir = tempfile::tempdir().expect("temp dir");

    ensure_builtin_preloaded_skills(dir.path()).unwrap();

    for builtin in BUILTIN_PRELOADED_SKILLS {
        let meta: serde_json::Value = serde_json::from_slice(
            &std::fs::read(dir.path().join(builtin.dir_name).join("_meta.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(meta["slug"].as_str(), Some(builtin.dir_name));
        assert_eq!(meta["version"].as_str(), Some("preloaded"));
        assert_eq!(meta["source"].as_str(), Some(builtin.source_url));
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn local_install_copies_clean_skill_and_rejects_existing_destination() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("source-skill");
    let skills_path = dir.path().join("skills");
    std::fs::create_dir_all(&skills_path).unwrap();
    write_clean_skill(&source, "Source");

    let (dest, scanned) =
        install_local_skill_source(source.to_str().unwrap(), &skills_path, &default_security())
            .unwrap();
    assert!(dest.join("SKILL.md").is_file());
    assert!(scanned >= 2);

    let err =
        install_local_skill_source(source.to_str().unwrap(), &skills_path, &default_security())
            .unwrap_err();
    assert!(err.to_string().contains("Destination skill already exists"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn local_install_rejects_missing_file_source_and_bad_destination_copy() {
    let dir = tempfile::tempdir().expect("temp dir");
    let skills_path = dir.path().join("skills");
    std::fs::create_dir_all(&skills_path).unwrap();

    let missing = install_local_skill_source("does-not-exist", &skills_path, &default_security())
        .unwrap_err();
    assert!(missing.to_string().contains("Source path does not exist"));

    let source_file = dir.path().join("not-a-directory");
    std::fs::write(&source_file, "# File").unwrap();
    let file_err = copy_dir_recursive_secure(&source_file, &skills_path.join("copy")).unwrap_err();
    assert!(file_err.to_string().contains("Skill source must be a directory"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn secure_copy_reports_missing_source_with_path_context() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing = dir.path().join("missing-source");

    let err = copy_dir_recursive_secure(&missing, &dir.path().join("dest")).unwrap_err();

    assert!(err.to_string().contains("failed to read metadata"));
    assert!(err.to_string().contains("missing-source"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn local_install_rejects_failed_security_audit_without_creating_destination() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("unsafe-skill");
    let skills_path = dir.path().join("skills");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(&skills_path).unwrap();
    std::fs::write(source.join("README.md"), "missing required skill manifest").unwrap();

    let require_manifest = SkillSecurityConfig { require_manifest: true, ..Default::default() };
    let err = install_local_skill_source(source.to_str().unwrap(), &skills_path, &require_manifest)
        .unwrap_err();

    assert!(err.to_string().contains("Skill security audit failed"));
    assert!(!skills_path.join("unsafe-skill").exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn secure_copy_recursively_copies_nested_regular_files() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("source");
    let dest = dir.path().join("dest");
    write_clean_skill(&source, "Copy");
    std::fs::create_dir_all(source.join("nested")).unwrap();
    std::fs::write(source.join("nested/info.txt"), "nested").unwrap();

    copy_dir_recursive_secure(&source, &dest).unwrap();

    assert_eq!(
        std::fs::read_to_string(dest.join("SKILL.md")).unwrap(),
        "# Copy\n\nSafe instructions."
    );
    assert_eq!(std::fs::read_to_string(dest.join("nested/info.txt")).unwrap(), "nested");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn remove_git_metadata_deletes_nested_git_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    let skill = dir.path().join("skill");
    std::fs::create_dir_all(skill.join(".git/objects")).unwrap();

    remove_git_metadata(&skill).unwrap();

    assert!(!skill.join(".git").exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn remove_git_metadata_is_noop_without_git_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    let skill = dir.path().join("skill");
    write_clean_skill(&skill, "No Metadata");

    remove_git_metadata(&skill).unwrap();

    assert!(skill.join("SKILL.md").is_file());
}

#[cfg(all(not(target_arch = "wasm32"), unix))]
#[test]
fn secure_copy_rejects_symlinked_skill_source_root() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("source");
    let linked_source = dir.path().join("linked-source");
    let dest = dir.path().join("dest");
    write_clean_skill(&source, "Linked Source");
    std::os::unix::fs::symlink(&source, &linked_source).unwrap();

    let err = copy_dir_recursive_secure(&linked_source, &dest).unwrap_err();

    assert!(err.to_string().contains("Refusing to copy symlinked skill source path"));
    assert!(!dest.exists());
}

#[cfg(all(not(target_arch = "wasm32"), unix))]
#[test]
// 安全复制在遍历前拒绝链接，防止复制阶段越出经过审计的源目录。
fn secure_copy_rejects_symlinks_inside_skill_source() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("source");
    let dest = dir.path().join("dest");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("SKILL.md"), "# Skill").unwrap();
    std::os::unix::fs::symlink(source.join("SKILL.md"), source.join("linked.md")).unwrap();

    let err = copy_dir_recursive_secure(&source, &dest).unwrap_err();

    assert!(err.to_string().contains("Refusing to copy symlink"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn git_install_clones_local_repo_removes_metadata_and_audits() {
    let _env_lock =
        crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner());
    let _git_guard = EnvGuard::set("VIBEWINDOW_TEST_PREFER_SYSTEM_GIT", "1");
    let dir = tempfile::tempdir().expect("temp dir");
    let repo = dir.path().join("repo-skill");
    let skills_path = dir.path().join("skills");
    write_clean_skill(&repo, "Repo Skill");
    std::fs::create_dir_all(&skills_path).unwrap();

    run_git(&repo, &["init"]);
    run_git(&repo, &["config", "user.email", "test@example.com"]);
    run_git(&repo, &["config", "user.name", "Test User"]);
    run_git(&repo, &["add", "SKILL.md"]);
    run_git(&repo, &["commit", "-m", "init"]);

    let (installed, scanned) =
        install_git_skill_source(repo.to_str().unwrap(), &skills_path, &default_security())
            .unwrap();

    assert_eq!(installed.file_name().and_then(|name| name.to_str()), Some("repo-skill"));
    assert!(installed.join("SKILL.md").is_file());
    assert!(!installed.join(".git").exists());
    assert!(scanned >= 1);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn skills_sh_install_rejects_malformed_source_before_checkout() {
    let dir = tempfile::tempdir().expect("temp dir");

    let err =
        install_skills_sh_source("https://skills.sh/owner/repo", dir.path(), &default_security())
            .unwrap_err();

    assert!(err.to_string().contains("invalid skills.sh source"));
    assert!(dir.path().read_dir().unwrap().next().is_none());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn handle_command_covers_list_audit_and_remove_paths() {
    let dir = tempfile::tempdir().expect("temp dir");
    let config =
        SkillRuntimeConfig { workspace_dir: dir.path().to_path_buf(), skills: Default::default() };
    let skills_path = skills_dir(dir.path());
    let skill = skills_path.join("remove-me");
    write_clean_skill(&skill, "Remove Me");

    handle_command(SkillCommands::List, &config).unwrap();
    handle_command(SkillCommands::Audit { source: skill.to_string_lossy().to_string() }, &config)
        .unwrap();
    let missing_audit =
        handle_command(SkillCommands::Audit { source: "missing".to_string() }, &config)
            .unwrap_err();
    assert!(missing_audit.to_string().contains("not found"));

    let invalid_remove =
        handle_command(SkillCommands::Remove { name: "../bad".to_string() }, &config).unwrap_err();
    assert!(invalid_remove.to_string().contains("Invalid skill name"));

    handle_command(SkillCommands::Remove { name: "remove-me".to_string() }, &config).unwrap();
    assert!(!skill.exists());
    let missing_remove =
        handle_command(SkillCommands::Remove { name: "remove-me".to_string() }, &config)
            .unwrap_err();
    assert!(missing_remove.to_string().contains("Skill not found"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn handle_command_audit_uses_default_disabled_security_policy() {
    let dir = tempfile::tempdir().expect("temp dir");
    let config =
        SkillRuntimeConfig { workspace_dir: dir.path().to_path_buf(), skills: Default::default() };
    let name = "task-0179-unsafe-installed-skill";
    let unsafe_skill = skills_dir(dir.path()).join(name);
    std::fs::create_dir_all(&unsafe_skill).unwrap();
    std::fs::write(unsafe_skill.join("README.md"), "missing manifest").unwrap();

    handle_command(SkillCommands::Audit { source: name.to_string() }, &config).unwrap();
    assert!(unsafe_skill.exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn handle_command_remove_rejects_path_separator_variants() {
    let dir = tempfile::tempdir().expect("temp dir");
    let config =
        SkillRuntimeConfig { workspace_dir: dir.path().to_path_buf(), skills: Default::default() };

    for name in ["..", "nested/skill", "nested\\skill"] {
        let err =
            handle_command(SkillCommands::Remove { name: name.to_string() }, &config).unwrap_err();
        assert!(err.to_string().contains("Invalid skill name"), "name: {name}");
    }
}

#[cfg(all(not(target_arch = "wasm32"), unix))]
#[test]
fn handle_command_remove_rejects_symlink_escape() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    let skills = skills_dir(&workspace);
    let outside = dir.path().join("outside-skill");
    std::fs::create_dir_all(&skills).unwrap();
    write_clean_skill(&outside, "Outside");
    std::os::unix::fs::symlink(&outside, skills.join("escaped")).unwrap();
    let config = SkillRuntimeConfig { workspace_dir: workspace, skills: Default::default() };

    let err =
        handle_command(SkillCommands::Remove { name: "escaped".to_string() }, &config).unwrap_err();

    assert!(err.to_string().contains("Skill path escapes skills directory"));
    assert!(outside.join("SKILL.md").is_file());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn handle_command_installs_local_skill_source() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("install-me");
    write_clean_skill(&source, "Install Me");
    let config = SkillRuntimeConfig {
        workspace_dir: dir.path().join("workspace"),
        skills: Default::default(),
    };

    handle_command(
        SkillCommands::Install { source: source.to_string_lossy().to_string() },
        &config,
    )
    .unwrap();

    assert!(skills_dir(&config.workspace_dir).join("install-me/SKILL.md").is_file());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn handle_command_trims_local_install_source() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("trimmed-source");
    write_clean_skill(&source, "Trimmed Source");
    let config = SkillRuntimeConfig {
        workspace_dir: dir.path().join("workspace"),
        skills: Default::default(),
    };

    handle_command(SkillCommands::Install { source: format!("  {}  ", source.display()) }, &config)
        .unwrap();

    assert!(skills_dir(&config.workspace_dir).join("trimmed-source/SKILL.md").is_file());
}
