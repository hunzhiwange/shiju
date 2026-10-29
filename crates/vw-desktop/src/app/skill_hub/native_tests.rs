//! 原生技能准备的真实临时文件、账户切换及目录边界回归。

use super::*;
use std::fs;
use zip::write::SimpleFileOptions;

fn credential(value: &str) -> SecretSkey {
    SecretSkey::new(value).unwrap()
}

fn archive() -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    {
        let mut writer = zip::ZipWriter::new(file.as_file_mut());
        writer.start_file("shiju/SKILL.md", SimpleFileOptions::default()).unwrap();
        writer.write_all(b"---\nname: shiju\nversion: 3.0.3\n---\n# shiju\n").unwrap();
        writer.finish().unwrap();
    }
    file
}

fn native_dir(home: &Path) -> PathBuf {
    vw_config_types::paths::home_config_dir(home).join("skills/shiju")
}

#[tokio::test]
async fn native_shiju_skill_installs_and_syncs_only_native_directory() {
    let home = tempfile::tempdir().unwrap();
    let other_roots = [
        ".agents",
        ".codex",
        ".workbuddy",
        ".claude",
        ".config/opencode",
        ".gemini",
        ".copilot",
        ".cursor",
        ".codeium/windsurf",
        "project/.shiju-agent",
        "project/.shiju-agent-dev",
    ];
    for root in other_roots {
        let directory = home.path().join(root).join("skills/shiju");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join(".env"), "SHIJU_KEY=unchanged\n").unwrap();
    }
    let state = NativeShijuSkillState::default();
    state.set_credential(Some(credential("active-secret")));
    let prepared =
        prepare_native_shiju_skill(state, home.path().to_path_buf(), |_| async { Ok(archive()) })
            .await
            .unwrap();
    assert!(prepared.installation.is_some());
    assert_eq!(
        fs::read_to_string(native_dir(home.path()).join(".env")).unwrap(),
        "SHIJU_KEY=active-secret\n"
    );
    for root in other_roots {
        let directory = home.path().join(root).join("skills/shiju");
        assert_eq!(fs::read_to_string(directory.join(".env")).unwrap(), "SHIJU_KEY=unchanged\n");
        assert!(!directory.join("SKILL.md").exists());
    }
}

#[tokio::test]
async fn native_shiju_skill_existing_install_syncs_without_download_or_upgrade() {
    let home = tempfile::tempdir().unwrap();
    install_shiju_archive_at_home(archive().path(), home.path()).unwrap();
    let state = NativeShijuSkillState::default();
    state.set_credential(Some(credential("new-secret")));
    let prepared = prepare_native_shiju_skill(state, home.path().to_path_buf(), |_| async {
        anyhow::bail!("existing skills must not download")
    })
    .await
    .unwrap();
    assert_eq!(prepared.installation.unwrap().version.as_deref(), Some("3.0.3"));
    assert_eq!(
        fs::read_to_string(native_dir(home.path()).join(".env")).unwrap(),
        "SHIJU_KEY=new-secret\n"
    );
}

#[tokio::test]
async fn native_shiju_skill_anonymous_startup_defers_download_until_login() {
    let home = tempfile::tempdir().unwrap();
    let state = NativeShijuSkillState::default();
    let prepared =
        prepare_native_shiju_skill(state.clone(), home.path().to_path_buf(), |_| async {
            anyhow::bail!("anonymous startup must not download")
        })
        .await
        .unwrap();
    assert!(prepared.installation.is_none());
    assert!(!native_dir(home.path()).exists());
    state.set_credential(Some(credential("login-secret")));
    prepare_native_shiju_skill(state, home.path().to_path_buf(), |_| async { Ok(archive()) })
        .await
        .unwrap();
    assert_eq!(
        fs::read_to_string(native_dir(home.path()).join(".env")).unwrap(),
        "SHIJU_KEY=login-secret\n"
    );
}

#[tokio::test]
async fn native_shiju_skill_switch_during_download_writes_current_credential() {
    let home = tempfile::tempdir().unwrap();
    let state = NativeShijuSkillState::default();
    state.set_credential(Some(credential("old-secret")));
    let switched = state.clone();
    let prepared = prepare_native_shiju_skill(
        state.clone(),
        home.path().to_path_buf(),
        move |key| async move {
            assert_eq!(key.expose(), "old-secret");
            switched.set_credential(Some(credential("current-secret")));
            Ok(archive())
        },
    )
    .await
    .unwrap();
    assert_eq!(prepared.revision, state.revision());
    assert_eq!(
        fs::read_to_string(native_dir(home.path()).join(".env")).unwrap(),
        "SHIJU_KEY=current-secret\n"
    );
    assert!(!format!("{state:?}").contains("current-secret"));
}

#[tokio::test]
async fn native_shiju_skill_sign_out_during_download_does_not_write_old_credential() {
    let home = tempfile::tempdir().unwrap();
    let state = NativeShijuSkillState::default();
    state.set_credential(Some(credential("old-secret")));
    let signed_out = state.clone();
    prepare_native_shiju_skill(state, home.path().to_path_buf(), move |_| async move {
        signed_out.set_credential(None);
        Ok(archive())
    })
    .await
    .unwrap();
    assert!(native_dir(home.path()).join("SKILL.md").is_file());
    assert!(!native_dir(home.path()).join(".env").exists());
}

#[tokio::test]
async fn native_shiju_skill_download_failure_leaves_no_partial_install() {
    let home = tempfile::tempdir().unwrap();
    let state = NativeShijuSkillState::default();
    state.set_credential(Some(credential("active-secret")));
    let revision = state.revision();
    let result = prepare_native_shiju_skill(state.clone(), home.path().to_path_buf(), |_| async {
        anyhow::bail!("download unavailable")
    })
    .await;
    assert!(result.is_err());
    assert!(!native_dir(home.path()).exists());
    assert_eq!(revision, state.revision());
}

#[tokio::test]
async fn native_shiju_skill_preserves_install_created_during_download() {
    let home = tempfile::tempdir().unwrap();
    let install_home = home.path().to_path_buf();
    let state = NativeShijuSkillState::default();
    state.set_credential(Some(credential("active-secret")));
    prepare_native_shiju_skill(state, home.path().to_path_buf(), move |_| async move {
        install_shiju_archive_at_home(archive().path(), &install_home).unwrap();
        fs::write(native_dir(&install_home).join("marker"), "keep").unwrap();
        Ok(archive())
    })
    .await
    .unwrap();
    assert_eq!(fs::read_to_string(native_dir(home.path()).join("marker")).unwrap(), "keep");
}

#[test]
fn native_shiju_skill_deduplicates_requests_and_rejects_stale_completions() {
    let state = NativeShijuSkillState::default();
    let (first, _) = state.begin().unwrap();
    assert!(state.begin().is_none());
    assert!(state.finish(first));
    let (second, _) = state.begin().unwrap();
    assert!(!state.finish(first));
    assert!(state.begin().is_none());
    assert!(state.finish(second));
}

#[cfg(unix)]
#[tokio::test]
async fn native_shiju_skill_rejects_symlink_before_download() {
    let home = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(
        outside.path(),
        vw_config_types::paths::home_config_dir(home.path()),
    )
    .unwrap();
    let state = NativeShijuSkillState::default();
    state.set_credential(Some(credential("active-secret")));
    let result = prepare_native_shiju_skill(state, home.path().to_path_buf(), |_| async {
        anyhow::bail!("unsafe paths must not download")
    })
    .await;
    assert!(result.unwrap_err().to_string().contains("符号链接"));
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
}
