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
