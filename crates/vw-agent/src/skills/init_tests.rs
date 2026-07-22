use super::*;
use std::fs;

#[test]
fn init_creates_skills_directory_and_readme() {
    let dir = tempfile::tempdir().expect("temp dir");

    init_skills_dir(dir.path()).expect("skills dir should initialize");

    let skills = crate::app::agent::skills::skills_dir(dir.path());
    assert!(skills.is_dir());
    assert!(skills.join("README.md").is_file());
    assert!(skills.join("find-skills").join("SKILL.md").is_file());
    assert!(skills.join("skill-creator").join("SKILL.md").is_file());
    assert!(skills.join(".download-policy.toml").is_file());

    let readme = fs::read_to_string(skills.join("README.md")).expect("readme content");
    assert!(readme.contains("# VibeWindow Skills"));
    assert!(readme.contains("SKILL.toml format"));
    assert!(readme.contains("vibewindow skills install <source>"));
}

#[test]
fn init_preserves_existing_readme() {
    let dir = tempfile::tempdir().expect("temp dir");
    let skills = crate::app::agent::skills::skills_dir(dir.path());
    fs::create_dir_all(&skills).expect("skills dir");
    fs::write(skills.join("README.md"), "custom readme").expect("custom readme");

    init_skills_dir(dir.path()).expect("skills dir should initialize");

    let readme = fs::read_to_string(skills.join("README.md")).expect("readme content");
    assert_eq!(readme, "custom readme");
    assert!(skills.join("find-skills").join("SKILL.md").is_file());
    assert!(skills.join("skill-creator").join("SKILL.md").is_file());
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
    fs::create_dir_all(skills.join("find-skills")).expect("find-skills dir");
    fs::create_dir_all(skills.join("skill-creator")).expect("skill-creator dir");
    fs::write(skills.join("README.md"), "custom readme").expect("custom readme");
    fs::create_dir_all(skills.join(".download-policy.toml")).expect("policy dir");

    let err = init_skills_dir(dir.path()).expect_err("policy directory should fail");

    assert!(err.to_string().contains("failed to read skill download policy"));
}
