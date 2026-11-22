use super::*;
fn identity(session: &str) -> RelayIdentity {
    RelayIdentity {
        tenant: vw_api_transport::TenantScope::new("profile-test", "tenant").unwrap(),
        project: "p".into(), session: session.into(),
        provider: "provider".into(), model: "model".into(), generation: 0,
    }
}

#[test]
fn acp_model_source_launch_profile_private_files_child_env_and_revocation() {
    let config = AcpAgentConfig { command: "python3".into(), ..Default::default() };
    let mut profile = AcpLaunchProfile::new(&config, identity("one")).unwrap();
    let binding = profile.binding.clone();
    assert!(binding.expires().is_none());
    let token = binding.bearer_token().to_owned();
    let directory = profile.directory().to_owned();
    let path = profile.write_config("config.json", token.as_bytes()).unwrap();
    assert!(profile.write_config("config.json", b"overwrite").is_err());
    assert!(profile.write_config("../escape", b"bad").is_err());
    assert!(profile.overlay_environment(HashMap::from([("HOME".into(), "/tmp".into())])).is_err());
    profile
        .overlay_environment(HashMap::from([("TASK9_LAUNCH_TOKEN".into(), token.clone())]))
        .unwrap();
    assert!(
        profile
            .overlay_environment(HashMap::from([("TASK9_LAUNCH_TOKEN".into(), "other".into())]))
            .is_err()
    );
    assert!(profile.append_arguments(&[token.clone()]).is_err());
    profile.append_arguments(&["--config".into(), path.to_string_lossy().into()]).unwrap();
    let output = std::process::Command::new("python3")
        .args(["-c", "import os,sys; assert open(sys.argv[1]).read()==os.environ['TASK9_LAUNCH_TOKEN']; print('PRIVATE_LAUNCH_OK')"])
        .arg(&path).envs(&profile.config().env).output().unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "PRIVATE_LAUNCH_OK");
    assert!(std::env::var("TASK9_LAUNCH_TOKEN").is_err());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(std::fs::metadata(&directory).unwrap().permissions().mode() & 0o777, 0o700);
    }
    assert!(binding.accepts(&token, &identity("one")));
    assert!(!binding.accepts(&token, &identity("two")));
    drop(profile);
    assert!(!directory.exists());
    assert!(!binding.accepts(&token, &identity("one")));
    let restored = AcpLaunchProfile::new(&config, identity("one")).unwrap();
    assert_ne!(token, restored.binding.bearer_token());
    assert!(!restored.binding.accepts(&token, &identity("one")));
}

#[test]
fn acp_model_source_launch_profile_references_separate_every_identity_boundary() {
    let original = identity("one");
    let profile = AcpLaunchProfile::new(&Default::default(), original.clone()).unwrap();
    let reference = &profile.config().env[REFERENCE_ENV];
    for boundary in 0..6 {
        let mut changed = original.clone();
        match boundary {
            0 => changed.session = "two".into(),
            1 => changed.project = "other-project".into(),
            2 => changed.provider = "other-provider".into(),
            3 => changed.model = "other-model".into(),
            4 => changed.generation += 1,
            _ => changed.tenant = vw_api_transport::TenantScope::new("other", "tenant").unwrap(),
        }
        let other = AcpLaunchProfile::new(&Default::default(), changed).unwrap();
        assert_ne!(reference, &other.config().env[REFERENCE_ENV]);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn acp_platform_private_profile_rejects_unsafe_temporary_parent() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    const CHILD: &str = "SHIJU_ACP_TEST_UNSAFE_TEMP_PARENT";
    if std::env::var_os(CHILD).is_some() {
        assert!(AcpLaunchProfile::new(&Default::default(), identity("temporary")).is_err());
        return;
    }

    let root = tempfile::tempdir().unwrap();
    let parent = root.path().join("temporary");
    std::fs::create_dir(&parent).unwrap();
    let linked = root.path().join("linked");
    symlink(&parent, &linked).unwrap();

    for (candidate, mode) in [(&parent, 0o777), (&linked, 0o700)] {
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(mode)).unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("acp_platform_private_profile_rejects_unsafe_temporary_parent")
            .env(CHILD, "1")
            .env("TMPDIR", candidate)
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(std::fs::read_dir(&parent).unwrap().count(), 0);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn acp_platform_private_profile_config_fails_closed_after_permission_change() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let mut profile = AcpLaunchProfile::new(&Default::default(), identity("config")).unwrap();
    let directory = profile.directory().to_owned();
    let metadata = directory.symlink_metadata().unwrap();
    assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
    assert_eq!(metadata.mode() & 0o7777, 0o700);
    let binding = profile.binding.clone();
    let token = binding.bearer_token();
    let config = profile.write_config("config", token.as_bytes()).unwrap();
    let metadata = config.symlink_metadata().unwrap();
    assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
    assert_eq!(metadata.mode() & 0o7777, 0o600);
    assert!(profile.append_arguments(&[token.to_owned()]).is_err());
    assert!(binding.accepts(token, &identity("config")));
    assert!(!binding.accepts(token, &identity("other")));

    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(profile.write_config("secret", b"test credential").is_err());
    assert!(!directory.join("secret").exists());
    drop(profile);
    assert!(!binding.accepts(token, &identity("config")));
    assert!(!directory.exists());
}

#[cfg(target_os = "linux")]
#[test]
fn acp_platform_private_profile_resumes_only_verified_history() {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt, symlink};

    let root = tempfile::tempdir().unwrap();
    let profile_for = |session| {
        let mut profile = AcpLaunchProfile::new(&Default::default(), identity(session)).unwrap();
        profile.state_root = Some(root.path().to_owned());
        profile
    };
    let profile = profile_for("history");
    let namespace = root.path().join("acp-claude-state");
    // An interrupted creation can leave the namespace without a session directory.
    std::fs::DirBuilder::new().mode(0o700).create(&namespace).unwrap();
    let history = profile.claude_state_directory().unwrap();
    assert_eq!(history.parent(), Some(namespace.as_path()));
    let metadata = history.symlink_metadata().unwrap();
    assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
    assert_eq!(metadata.mode() & 0o7777, 0o700);
    let marker = history.join("history");
    std::fs::write(&marker, b"saved").unwrap();
    let old_token = profile.binding.bearer_token().to_owned();
    drop(profile);

    let restored = profile_for("history");
    assert_eq!(restored.claude_state_directory().unwrap(), history);
    assert_eq!(std::fs::read(&marker).unwrap(), b"saved");
    assert!(!restored.binding.accepts(&old_token, &identity("history")));
    assert_ne!(profile_for("other").claude_state_directory().unwrap(), history);

    std::fs::set_permissions(&history, std::fs::Permissions::from_mode(0o750)).unwrap();
    assert!(restored.claude_state_directory().is_err());
    assert_eq!(history.symlink_metadata().unwrap().mode() & 0o7777, 0o750);

    std::fs::set_permissions(&namespace, std::fs::Permissions::from_mode(0o770)).unwrap();
    assert!(profile_for("new").claude_state_directory().is_err());
    assert_eq!(std::fs::read_dir(&namespace).unwrap().count(), 2);

    let linked = root.path().join("linked");
    symlink(root.path(), &linked).unwrap();
    let mut linked_profile = profile_for("linked");
    linked_profile.state_root = Some(linked);
    std::fs::set_permissions(&namespace, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(linked_profile.claude_state_directory().is_err());
    assert_eq!(std::fs::read_dir(&namespace).unwrap().count(), 2);

    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o777)).unwrap();
    assert!(profile_for("unsafe-root").claude_state_directory().is_err());
    assert_eq!(std::fs::read_dir(&namespace).unwrap().count(), 2);
}

#[cfg(target_os = "linux")]
#[test]
fn acp_platform_private_profile_rejects_symlinked_history_namespace() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let mut profile = AcpLaunchProfile::new(&Default::default(), identity("linked")).unwrap();
    profile.state_root = Some(root.path().to_owned());
    symlink(profile.directory(), root.path().join("acp-claude-state")).unwrap();
    assert!(profile.claude_state_directory().is_err());
    assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
}
