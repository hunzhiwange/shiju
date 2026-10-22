//! 技能安装器测试，覆盖归档校验、目标目录解析、版本检测和卸载边界。
//! 路径穿越、符号链接及无效清单必须在替换现有安装前失败，确保操作可控。

use super::installer::{
    ShijuKeySyncOutcome, SkillInstallScope, SkillInstallSelection, SkillInstallTarget,
    install_shiju_archive_at_home, install_shiju_archive_for_targets, resolve_skill_destinations,
    shiju_installation_at_home, shiju_installations_for_targets, sync_shiju_key_at_home,
    sync_shiju_key_for_targets, uninstall_shiju_at_home, uninstall_shiju_for_targets,
    version_is_newer,
};
use super::{ShijuPackageRelease, SkillDownloadUrl};
use crate::app::account::SecretSkey;
use std::fs;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

fn write_archive(path: &Path, entries: &[(&str, u32, &str)]) {
    let file = fs::File::create(path).expect("create fixture archive");
    let mut archive = ZipWriter::new(file);
    for (name, mode, content) in entries {
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .unix_permissions(*mode);
        archive.start_file(*name, options).expect("start fixture entry");
        archive.write_all(content.as_bytes()).expect("write fixture entry");
    }
    archive.finish().expect("finish fixture archive");
}

fn target(home: &Path, config_dir: &str) -> std::path::PathBuf {
    home.join(config_dir).join("skills").join("shiju")
}

fn other_config_dir() -> &'static str {
    if vw_config_types::paths::HOME_CONFIG_DIR_NAME
        == vw_config_types::paths::DEV_HOME_CONFIG_DIR_NAME
    {
        vw_config_types::paths::PROD_HOME_CONFIG_DIR_NAME
    } else {
        vw_config_types::paths::DEV_HOME_CONFIG_DIR_NAME
    }
}

#[test]
fn valid_archive_installs_only_current_environment() {
    let temp = tempfile::tempdir().expect("fixture home");
    let home = temp.path();
    for config_dir in [vw_config_types::paths::HOME_CONFIG_DIR_NAME, other_config_dir()] {
        let existing = target(home, config_dir);
        fs::create_dir_all(&existing).expect("create existing install");
        fs::write(existing.join("old.txt"), "old").expect("write old install");
    }
    let archive_path = home.join("shiju.zip");
    write_archive(
        &archive_path,
        &[("shiju/SKILL.md", 0o100644, "# shiju"), ("shiju/bin/shiju", 0o100755, "binary")],
    );

    let report =
        install_shiju_archive_at_home(&archive_path, home).expect("install valid shiju archive");

    let installed = target(home, vw_config_types::paths::HOME_CONFIG_DIR_NAME);
    assert_eq!(report.installed_dir, installed);
    assert_eq!(
        fs::read_to_string(installed.join("SKILL.md")).expect("read installed skill"),
        "# shiju"
    );
    assert!(!installed.join("old.txt").exists());
    assert_eq!(
        fs::read_to_string(target(home, other_config_dir()).join("old.txt"))
            .expect("other environment remains unchanged"),
        "old"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(installed.join("bin/shiju"))
            .expect("read executable metadata")
            .permissions()
            .mode();
        assert_ne!(mode & 0o100, 0);
    }
}

#[test]
fn unsafe_or_invalid_archive_leaves_existing_installs_unchanged() {
    let temp = tempfile::tempdir().expect("fixture home");
    let home = temp.path();
    for config_dir in [".shiju-agent", ".shiju-agent-dev"] {
        let existing = target(home, config_dir);
        fs::create_dir_all(&existing).expect("create existing install");
        fs::write(existing.join("marker.txt"), config_dir).expect("write marker");
    }
    let archive_path = home.join("unsafe.zip");
    write_archive(
        &archive_path,
        &[("shiju/SKILL.md", 0o100644, "# shiju"), ("../escaped.txt", 0o100644, "escape")],
    );

    let error =
        install_shiju_archive_at_home(&archive_path, home).expect_err("reject unsafe archive");

    assert!(error.to_string().contains("不安全"));
    assert!(!home.join("escaped.txt").exists());
    for config_dir in [".shiju-agent", ".shiju-agent-dev"] {
        assert_eq!(
            fs::read_to_string(target(home, config_dir).join("marker.txt"))
                .expect("read unchanged marker"),
            config_dir
        );
    }
}

#[test]
fn symlink_or_missing_skill_manifest_is_rejected() {
    let temp = tempfile::tempdir().expect("fixture home");
    let symlink_archive = temp.path().join("symlink.zip");
    let file = fs::File::create(&symlink_archive).expect("create symlink archive");
    let mut archive = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o755);
    archive.start_file("shiju/SKILL.md", options).expect("start manifest");
    archive.write_all(b"# shiju").expect("write manifest");
    archive.add_symlink("shiju/current", "/tmp/target", options).expect("add symlink");
    archive.finish().expect("finish symlink archive");
    assert!(
        install_shiju_archive_at_home(&symlink_archive, temp.path())
            .expect_err("reject symlink")
            .to_string()
            .contains("符号链接")
    );

    let missing_manifest_archive = temp.path().join("missing.zip");
    write_archive(&missing_manifest_archive, &[("shiju/README.md", 0o100644, "missing manifest")]);
    assert!(
        install_shiju_archive_at_home(&missing_manifest_archive, temp.path())
            .expect_err("reject missing manifest")
            .to_string()
            .contains("SKILL.md")
    );
}

#[test]
fn local_installation_reads_manifest_version_and_detects_newer_release() {
    let temp = tempfile::tempdir().expect("fixture home");
    let installed = target(temp.path(), vw_config_types::paths::HOME_CONFIG_DIR_NAME);
    fs::create_dir_all(&installed).expect("create installed skill");
    fs::write(installed.join("SKILL.md"), "---\nname: shiju\nversion: 3.0.3\n---\n# shiju\n")
        .expect("write skill manifest");

    let installation = shiju_installation_at_home(temp.path())
        .expect("read local installation")
        .expect("installation exists");

    assert_eq!(installation.installed_dir, installed);
    assert_eq!(installation.version.as_deref(), Some("3.0.3"));
    assert!(version_is_newer("3.0.3", "3.1.0"));
    assert!(!version_is_newer("3.1.0", "3.0.3"));
    assert!(!version_is_newer("3.0.3", "3.0.3"));

    let release = ShijuPackageRelease {
        download_url: SkillDownloadUrl::parse(
            "https://skill.shiju.cloud/hub/releases/3.1.0/shiju-darwin-arm64/shiju.zip?secret=x",
        )
        .expect("trusted release URL"),
    };
    assert_eq!(release.version(), Some("3.1.0"));
}

#[test]
fn uninstall_removes_only_current_environment_and_rejects_symlink() {
    let temp = tempfile::tempdir().expect("fixture home");
    let current = target(temp.path(), vw_config_types::paths::HOME_CONFIG_DIR_NAME);
    let other = target(temp.path(), other_config_dir());
    fs::create_dir_all(&current).expect("create current install");
    fs::create_dir_all(&other).expect("create other install");
    fs::write(current.join("SKILL.md"), "# current").expect("write current manifest");
    fs::write(other.join("SKILL.md"), "# other").expect("write other manifest");

    assert!(uninstall_shiju_at_home(temp.path()).expect("uninstall current"));
    assert!(!current.exists());
    assert!(other.is_dir());
    assert!(!uninstall_shiju_at_home(temp.path()).expect("already absent"));

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&other, &current).expect("create unsafe install symlink");
        assert!(
            uninstall_shiju_at_home(temp.path())
                .expect_err("reject symlink")
                .to_string()
                .contains("符号链接")
        );
        assert!(other.is_dir());
    }
}

#[test]
fn sync_shiju_key_updates_only_target_entry() {
    let temp = tempfile::tempdir().expect("fixture home");
    let skill_dir = temp.path().join(".workbuddy/skills/shiju");
    fs::create_dir_all(&skill_dir).expect("create installed skill");
    fs::write(
        skill_dir.join(".env"),
        "API_URL=https://example.test\nSHIJU_KEY=old\n# keep this\nSHIJU_KEY=duplicate\n",
    )
    .expect("write existing env");
    let skey = SecretSkey::new("replacement-secret").expect("valid skey");

    let outcome = sync_shiju_key_at_home(temp.path(), &skey).expect("sync key into existing env");

    assert_eq!(outcome, ShijuKeySyncOutcome::Updated);
    assert_eq!(
        fs::read_to_string(skill_dir.join(".env")).expect("read updated env"),
        "API_URL=https://example.test\nSHIJU_KEY=replacement-secret\n# keep this\n"
    );

    let append_home = tempfile::tempdir().expect("fixture append home");
    let append_skill_dir = append_home.path().join(".workbuddy/skills/shiju");
    fs::create_dir_all(&append_skill_dir).expect("create append skill");
    fs::write(append_skill_dir.join(".env"), "OTHER=value")
        .expect("write env without target entry");

    sync_shiju_key_at_home(append_home.path(), &skey).expect("append key to existing env");

    assert_eq!(
        fs::read_to_string(append_skill_dir.join(".env")).expect("read appended env"),
        "OTHER=value\nSHIJU_KEY=replacement-secret\n"
    );
}

#[test]
fn sync_shiju_key_creates_env_for_existing_skill() {
    let temp = tempfile::tempdir().expect("fixture home");
    let skill_dir = temp.path().join(".workbuddy/skills/shiju");
    fs::create_dir_all(&skill_dir).expect("create installed skill");
    let skey = SecretSkey::new("new-secret").expect("valid skey");

    let outcome =
        sync_shiju_key_at_home(temp.path(), &skey).expect("create env for installed skill");

    assert_eq!(outcome, ShijuKeySyncOutcome::Updated);
    let env_path = skill_dir.join(".env");
    assert_eq!(fs::read_to_string(&env_path).expect("read created env"), "SHIJU_KEY=new-secret\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(env_path).expect("read env metadata").permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn sync_shiju_key_rejects_unsafe_targets() {
    let missing_home = tempfile::tempdir().expect("fixture missing home");
    let skey = SecretSkey::new("never-print-this-secret").expect("valid skey");
    assert_eq!(
        sync_shiju_key_at_home(missing_home.path(), &skey).expect("skip missing skill"),
        ShijuKeySyncOutcome::SkillNotInstalled
    );
    assert!(!missing_home.path().join(".workbuddy").exists());

    let non_file_home = tempfile::tempdir().expect("fixture non-file home");
    let non_file_env = non_file_home.path().join(".workbuddy/skills/shiju/.env");
    fs::create_dir_all(&non_file_env).expect("create non-file env");
    let error = sync_shiju_key_at_home(non_file_home.path(), &skey)
        .expect_err("reject non-file env target");
    assert!(error.to_string().contains("普通文件"));
    assert!(!error.to_string().contains(skey.expose()));

    #[cfg(unix)]
    {
        let skill_symlink_home = tempfile::tempdir().expect("fixture skill symlink home");
        let skills_root = skill_symlink_home.path().join(".workbuddy/skills");
        let outside_skill = skill_symlink_home.path().join("outside-skill");
        fs::create_dir_all(&skills_root).expect("create skills root");
        fs::create_dir(&outside_skill).expect("create outside skill");
        std::os::unix::fs::symlink(&outside_skill, skills_root.join("shiju"))
            .expect("create unsafe skill symlink");

        let error = sync_shiju_key_at_home(skill_symlink_home.path(), &skey)
            .expect_err("reject skill directory symlink");

        assert!(error.to_string().contains("符号链接"));
        assert!(!outside_skill.join(".env").exists());

        let symlink_home = tempfile::tempdir().expect("fixture symlink home");
        let skill_dir = symlink_home.path().join(".workbuddy/skills/shiju");
        fs::create_dir_all(&skill_dir).expect("create installed skill");
        let outside = symlink_home.path().join("outside.env");
        fs::write(&outside, "UNCHANGED=true\n").expect("write symlink target");
        std::os::unix::fs::symlink(&outside, skill_dir.join(".env"))
            .expect("create unsafe env symlink");

        let error =
            sync_shiju_key_at_home(symlink_home.path(), &skey).expect_err("reject env symlink");

        assert!(error.to_string().contains("符号链接"));
        assert!(!error.to_string().contains(skey.expose()));
        assert_eq!(
            fs::read_to_string(outside).expect("read unchanged symlink target"),
            "UNCHANGED=true\n"
        );
    }
}

#[test]
fn install_targets_resolve_documented_global_and_project_paths() {
    let temp = tempfile::tempdir().expect("fixture roots");
    let home = temp.path().join("home");
    let project = temp.path().join("project");
    fs::create_dir(&home).expect("create home");
    fs::create_dir(&project).expect("create project");
    let targets = [
        SkillInstallTarget::VibeWindow,
        SkillInstallTarget::AgentSkills,
        SkillInstallTarget::Codex,
        SkillInstallTarget::WorkBuddy,
        SkillInstallTarget::ClaudeCode,
        SkillInstallTarget::OpenCode,
        SkillInstallTarget::GeminiCli,
        SkillInstallTarget::GitHubCopilot,
        SkillInstallTarget::Cursor,
        SkillInstallTarget::Windsurf,
    ];
    let global =
        targets.map(|target| SkillInstallSelection { target, scope: SkillInstallScope::Global });
    let project_scoped =
        targets.map(|target| SkillInstallSelection { target, scope: SkillInstallScope::Project });

    let global_destinations =
        resolve_skill_destinations(&home, Some(&project), &global).expect("resolve global targets");
    let project_destinations = resolve_skill_destinations(&home, Some(&project), &project_scoped)
        .expect("resolve project targets");

    let expected_global_roots = [
        home.join(vw_config_types::paths::HOME_CONFIG_DIR_NAME).join("skills"),
        home.join(".agents/skills"),
        home.join(".codex/skills"),
        home.join(".workbuddy/skills"),
        home.join(".claude/skills"),
        home.join(".config/opencode/skills"),
        home.join(".gemini/skills"),
        home.join(".copilot/skills"),
        home.join(".cursor/skills"),
        home.join(".codeium/windsurf/skills"),
    ];
    let expected_project_roots = [
        project.join(vw_config_types::paths::HOME_CONFIG_DIR_NAME).join("skills"),
        project.join(".agents/skills"),
        project.join(".codex/skills"),
        project.join(".workbuddy/skills"),
        project.join(".claude/skills"),
        project.join(".opencode/skills"),
        project.join(".gemini/skills"),
        project.join(".github/skills"),
        project.join(".cursor/skills"),
        project.join(".windsurf/skills"),
    ];
    assert_eq!(
        global_destinations
            .iter()
            .map(|destination| destination.skills_root.clone())
            .collect::<Vec<_>>(),
        expected_global_roots
    );
    assert_eq!(
        project_destinations
            .iter()
            .map(|destination| destination.skills_root.clone())
            .collect::<Vec<_>>(),
        expected_project_roots
    );
    assert!(
        global_destinations
            .iter()
            .chain(&project_destinations)
            .all(|destination| destination.installed_dir == destination.skills_root.join("shiju"))
    );
    assert!(
        resolve_skill_destinations(
            &home,
            None,
            &[SkillInstallSelection {
                target: SkillInstallTarget::AgentSkills,
                scope: SkillInstallScope::Project,
            }],
        )
        .expect_err("project scope requires a project root")
        .to_string()
        .contains("项目")
    );
}

#[test]
fn multi_target_install_commits_all_destinations_or_rolls_back() {
    let temp = tempfile::tempdir().expect("fixture roots");
    let home = temp.path().join("home");
    let project = temp.path().join("project");
    fs::create_dir(&home).expect("create home");
    fs::create_dir(&project).expect("create project");
    let archive_path = temp.path().join("shiju.zip");
    write_archive(
        &archive_path,
        &[("shiju/SKILL.md", 0o100644, "---\nname: shiju\nversion: 4.2.0\n---\n# shiju\n")],
    );
    let selections = [
        SkillInstallSelection {
            target: SkillInstallTarget::AgentSkills,
            scope: SkillInstallScope::Global,
        },
        SkillInstallSelection {
            target: SkillInstallTarget::ClaudeCode,
            scope: SkillInstallScope::Project,
        },
    ];
    for existing in [home.join(".agents/skills/shiju"), project.join(".claude/skills/shiju")] {
        fs::create_dir_all(&existing).expect("create existing install");
        fs::write(existing.join("old.txt"), "old").expect("write old install");
    }

    let report =
        install_shiju_archive_for_targets(&archive_path, &home, Some(&project), &selections)
            .expect("install every target");

    assert_eq!(report.installs.len(), 2);
    assert!(report.installs.iter().all(|install| install.version.as_deref() == Some("4.2.0")));
    for installed in [home.join(".agents/skills/shiju"), project.join(".claude/skills/shiju")] {
        assert!(installed.join("SKILL.md").is_file());
        assert!(!installed.join("old.txt").exists());
    }

    #[cfg(unix)]
    {
        let unsafe_home = temp.path().join("unsafe-home");
        fs::create_dir(&unsafe_home).expect("create unsafe home");
        let preserved = unsafe_home.join(".agents/skills/shiju");
        fs::create_dir_all(&preserved).expect("create preserved install");
        fs::write(preserved.join("marker.txt"), "preserved").expect("write marker");
        let outside = temp.path().join("outside");
        fs::create_dir(&outside).expect("create outside");
        std::os::unix::fs::symlink(&outside, unsafe_home.join(".claude"))
            .expect("create unsafe root symlink");

        let error = install_shiju_archive_for_targets(
            &archive_path,
            &unsafe_home,
            None,
            &[
                SkillInstallSelection {
                    target: SkillInstallTarget::AgentSkills,
                    scope: SkillInstallScope::Global,
                },
                SkillInstallSelection {
                    target: SkillInstallTarget::ClaudeCode,
                    scope: SkillInstallScope::Global,
                },
            ],
        )
        .expect_err("unsafe destination rejects the transaction");

        assert!(error.to_string().contains("符号链接"));
        assert_eq!(
            fs::read_to_string(preserved.join("marker.txt")).expect("read preserved marker"),
            "preserved"
        );
        assert!(!outside.join("skills/shiju").exists());
    }
}

#[test]
fn target_operations_reject_symlinks_and_deduplicate_paths() {
    let temp = tempfile::tempdir().expect("fixture home");
    let home = temp.path();
    let duplicate = SkillInstallSelection {
        target: SkillInstallTarget::AgentSkills,
        scope: SkillInstallScope::Global,
    };
    let destinations = resolve_skill_destinations(home, None, &[duplicate, duplicate])
        .expect("deduplicate identical destinations");
    assert_eq!(destinations.len(), 1);

    let installed = home.join(".agents/skills/shiju");
    fs::create_dir_all(&installed).expect("create installed target");
    fs::write(installed.join("SKILL.md"), "# shiju").expect("write manifest");
    let installations = shiju_installations_for_targets(home, None, &[duplicate, duplicate])
        .expect("detect deduplicated targets");
    assert_eq!(installations.targets.len(), 1);
    assert!(installations.targets[0].installation.is_some());

    #[cfg(unix)]
    {
        let outside = home.join("outside");
        fs::create_dir(&outside).expect("create outside");
        std::os::unix::fs::symlink(&outside, home.join(".workbuddy"))
            .expect("create unsafe root symlink");
        let unsafe_target = SkillInstallSelection {
            target: SkillInstallTarget::WorkBuddy,
            scope: SkillInstallScope::Global,
        };
        let selected = [duplicate, unsafe_target];

        let detection_error = shiju_installations_for_targets(home, None, &selected)
            .expect_err("detection rejects unsafe destination");
        assert!(detection_error.to_string().contains("符号链接"));

        let uninstall_error = uninstall_shiju_for_targets(home, None, &selected)
            .expect_err("uninstall rejects unsafe destination");
        assert!(uninstall_error.to_string().contains("符号链接"));
        assert!(installed.is_dir());

        let skey = SecretSkey::new("must-not-leak").expect("valid skey");
        let sync_error = sync_shiju_key_for_targets(home, None, &selected, &skey)
            .expect_err("sync rejects unsafe destination");
        assert!(sync_error.to_string().contains("符号链接"));
        assert!(!sync_error.to_string().contains(skey.expose()));
        assert!(!installed.join(".env").exists());
        assert!(!outside.join("skills/shiju/.env").exists());
    }
}
