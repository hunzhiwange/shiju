//! Outbound lifecycle invariants; real CLI acceptance is separate.
use super::{ParsedAcpOptions, session};
use serde_json::json;

#[tokio::test]
async fn acp_skill_mount_lifecycle_scanner_change_retires_only_own_cached_client() {
    let _guard = super::CACHE_LOCK.lock().await;
    let cwd = tempfile::tempdir().unwrap();
    let config =
        serde_json::from_value(json!({"command":"unused-lifecycle-agent", "args":[], "env":{}}))
            .unwrap();
    let options =
        ParsedAcpOptions { runtime_identity: Some("lifecycle-owner".into()), ..Default::default() };
    let old = session::get_cached_acp_client("gemini", &config, cwd.path(), &options);
    let mut other_options = options.clone();
    other_options.runtime_identity = Some("lifecycle-other".into());
    let other = session::get_cached_acp_client("gemini", &config, cwd.path(), &other_options);
    let mut changed = options.clone();
    changed.full_access = true;
    session::stop_incompatible_cached_clients("gemini", &config, cwd.path(), &changed)
        .await
        .unwrap();
    {
        let cache = super::ACP_CLIENT_CACHE.lock();
        assert!(!cache.contains_key(&old.key));
        assert!(cache.contains_key(&other.key));
    }
    session::close_cached_acp_client_if_unshared(&other).await;
}

#[test]
fn acp_skill_mount_lifecycle_busy_rejects_same_session_but_allows_other_session() {
    let cwd = tempfile::tempdir().unwrap();
    let directory = cwd.path().to_str().unwrap();
    let lease = super::lifecycle::try_lock(&Default::default(), "lifecycle-busy", directory).unwrap();
    assert!(super::lifecycle::try_lock(&Default::default(), "lifecycle-busy", directory).is_err());
    assert!(super::lifecycle::try_lock(&Default::default(), "lifecycle-independent", directory).is_ok());
    drop(lease);
    assert!(super::lifecycle::try_lock(&Default::default(), "lifecycle-busy", directory).is_ok());
}

#[tokio::test]
async fn acp_skill_mount_lifecycle_content_removal_and_session_change_identity() {
    let _guard = super::CACHE_LOCK.lock().await;
    let project = tempfile::tempdir().unwrap();
    let source = project.path().join("source");
    std::fs::create_dir(&source).unwrap();
    let file = source.join("SKILL.md");
    let v1 = "---\nname: lifecycle\ndescription: fixture\n---\nversion-one\n";
    let v2 = v1.replace("version-one", "version-two");
    assert_eq!(v1.len(), v2.len());
    std::fs::write(&file, v1).unwrap();
    let mounts = [acp_types::SessionSkillMount {
        name: "lifecycle".into(),
        path: source.to_str().unwrap().into(),
    }];
    let security = Default::default();
    let prepare = |session| {
        super::prepare(&mounts, project.path(), session, &security).unwrap().unwrap()
    };
    let first = prepare("lifecycle-first");
    let other = prepare("lifecycle-other");
    assert_ne!(first.0.fingerprint(), other.0.fingerprint());
    assert_ne!(first.0.root(), other.0.root());
    std::fs::write(&file, v2).unwrap();
    assert!(first.0.validate().is_err());
    let updated = prepare("lifecycle-first");
    assert_ne!(first.0.fingerprint(), updated.0.fingerprint());
    let config =
        serde_json::from_value(json!({"command":"unused-lifecycle-agent", "args":[], "env":{}}))
            .unwrap();
    let cached = |view: Option<super::MountedSkills>| {
        let options = ParsedAcpOptions {
            runtime_identity: Some("lifecycle-first".into()),
            skill_mounts: if view.is_some() { mounts.to_vec() } else { vec![] },
            skill_view: view,
            ..Default::default()
        };
        session::get_cached_acp_client("gemini", &config, project.path(), &options)
    };
    let original_client = cached(Some(first.clone()));
    let updated_client = cached(Some(updated.clone()));
    let other_client = cached(Some(other));
    let cleared_client = cached(None);
    assert_ne!(original_client.key, updated_client.key);
    assert_ne!(original_client.key, other_client.key);
    assert_ne!(updated_client.key, cleared_client.key);
    for client in [original_client, updated_client, other_client, cleared_client] {
        session::close_cached_acp_client_if_unshared(&client).await;
    }
    std::fs::write(source.join("SKILL.disabled"), "").unwrap();
    assert!(updated.0.validate().is_err());
    assert!(
        super::prepare(&mounts, project.path(), "lifecycle-first", &security)
            .is_err()
    );
    std::fs::remove_file(source.join("SKILL.disabled")).unwrap();
    assert!(
        super::prepare(&[], project.path(), "lifecycle-first", &security)
            .unwrap()
            .is_none()
    );
    std::fs::remove_file(&file).unwrap();
    assert!(updated.0.validate().is_err());
    assert!(
        super::prepare(&mounts, project.path(), "lifecycle-first", &security)
            .is_err()
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn acp_platform_mount_lifecycle_restore_retires_replaced_cached_view() {
    let _guard = super::CACHE_LOCK.lock().await;
    let project = tempfile::tempdir().unwrap();
    let source = project.path().join("外部技能");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(
        source.join("SKILL.md"),
        "---\nname: external\ndescription: fixture\n---\nexternal marker\n",
    )
    .unwrap();
    let mounts = vec![acp_types::SessionSkillMount {
        name: "external".into(),
        path: source.to_str().unwrap().into(),
    }];
    let owner = "platform-mount-restore";
    let options = || ParsedAcpOptions {
        runtime_identity: Some(owner.into()),
        skill_mounts: mounts.clone(),
        skill_view: super::prepare(&mounts, project.path(), owner, &Default::default()).unwrap(),
        ..Default::default()
    };
    let original = options();
    let config =
        serde_json::from_value(json!({"command":"unused-lifecycle-agent", "args":[], "env":{}}))
            .unwrap();
    let cached = session::get_cached_acp_client("fixture", &config, project.path(), &original);
    let old_root = original.skill_view.as_ref().unwrap().0.root();
    std::fs::rename(old_root, old_root.with_file_name("retired")).unwrap();
    std::fs::create_dir(old_root).unwrap();
    std::os::unix::fs::symlink(source.canonicalize().unwrap(), old_root.join("external")).unwrap();

    let restored = options();
    session::stop_incompatible_cached_clients("fixture", &config, project.path(), &restored)
        .await
        .unwrap();
    let replacement = session::get_cached_acp_client("fixture", &config, project.path(), &restored);
    assert_eq!(cached.key, replacement.key);
    assert!(!std::sync::Arc::ptr_eq(&cached.client, &replacement.client));
    let cleared = ParsedAcpOptions { runtime_identity: Some(owner.into()), ..Default::default() };
    session::stop_incompatible_cached_clients("fixture", &config, project.path(), &cleared)
        .await
        .unwrap();
    assert!(!super::ACP_CLIENT_CACHE.lock().contains_key(&replacement.key));
}
