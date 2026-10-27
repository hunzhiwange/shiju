//! 验证技能初始化生成的目录结构、清单内容与冲突处理。
//! 初始化只能补齐缺失资产；已有 README 必须保留，路径为文件时返回明确错误。

use super::*;
use std::fs;

#[test]
fn init_creates_skills_directory_and_readme() {
    let dir = tempfile::tempdir().expect("temp dir");

    init_skills_dir(dir.path()).expect("skills dir should initialize");

    let skills = crate::app::agent::skills::skills_dir(dir.path());
    assert!(skills.is_dir());
    assert!(skills.join("README.md").is_file());
    assert!(skills.join("create-task-plans").join("SKILL.md").is_file());
    assert!(skills.join("do-task-plans").join("SKILL.md").is_file());
    assert!(skills.join("rustcodegraph").join("SKILL.md").is_file());
    assert!(skills.join(".download-policy.toml").is_file());

    let readme = fs::read_to_string(skills.join("README.md")).expect("readme content");
    assert!(readme.contains("# VibeWindow Skills"));
    assert!(readme.contains("SKILL.toml format"));
    assert!(readme.contains("shiju skills install <source>"));
}

#[test]
// 初始化只补缺失文件，用户维护的 README 内容和权限不应被模板覆盖。
fn init_preserves_existing_readme() {
    let dir = tempfile::tempdir().expect("temp dir");
    let skills = crate::app::agent::skills::skills_dir(dir.path());
    fs::create_dir_all(&skills).expect("skills dir");
    fs::write(skills.join("README.md"), "custom readme").expect("custom readme");

    init_skills_dir(dir.path()).expect("skills dir should initialize");

    let readme = fs::read_to_string(skills.join("README.md")).expect("readme content");
    assert_eq!(readme, "custom readme");
    assert!(skills.join("create-task-plans").join("SKILL.md").is_file());
    assert!(skills.join("do-task-plans").join("SKILL.md").is_file());
    assert!(skills.join("rustcodegraph").join("SKILL.md").is_file());
    assert!(skills.join(".download-policy.toml").is_file());
}

#[test]
fn init_returns_error_when_skills_path_is_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let skills = crate::app::agent::skills::skills_dir(dir.path());
    fs::write(&skills, "not a directory").expect("skills file");

    let err = init_skills_dir(dir.path()).expect_err("skills path file should fail");

    assert!(!err.to_string().is_empty());
}

#[cfg(unix)]
#[test]
fn init_returns_error_when_readme_write_fails() {
    let dir = tempfile::tempdir().expect("temp dir");
    let skills = crate::app::agent::skills::skills_dir(dir.path());
    fs::create_dir_all(&skills).expect("skills dir");
    std::os::unix::fs::symlink(
        dir.path().join("missing").join("README.md"),
        skills.join("README.md"),
    )
    .expect("dangling readme symlink");

    let err = init_skills_dir(dir.path()).expect_err("dangling readme symlink should fail");

    assert!(!err.to_string().is_empty());
}

#[test]
fn init_returns_error_when_policy_path_is_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    let skills = crate::app::agent::skills::skills_dir(dir.path());
    fs::create_dir_all(skills.join("create-task-plans")).expect("create-task-plans dir");
    fs::create_dir_all(skills.join("do-task-plans")).expect("do-task-plans dir");
    fs::create_dir_all(skills.join("rustcodegraph")).expect("rustcodegraph dir");
    fs::write(skills.join("README.md"), "custom readme").expect("custom readme");
    fs::create_dir_all(skills.join(".download-policy.toml")).expect("policy dir");

    let err = init_skills_dir(dir.path()).expect_err("policy directory should fail");

    assert!(err.to_string().contains("failed to read skill download policy"));
}
