use super::*;
use crate::acp::skill_mounts::MountedSkills;

fn profile() -> AcpLaunchProfile {
    let launch = AcpAgentConfig { command: "mimo".into(), ..Default::default() };
    AcpLaunchProfile::new(
        &launch,
        model_transport::RelayIdentity {
            tenant: Default::default(),
            project: "project".into(),
            session: "session:mimo-code".into(),
            provider: "fixture".into(),
            model: "model".into(),
            generation: 0,
        },
    )
    .unwrap()
}

#[test]
fn acp_skill_coexistence_mimo_code_empty_selection_preserves_config() {
    let profile = profile();
    let mut config = profile.config().clone();
    configure(&profile, &mut config, None::<&MountedSkills>).unwrap();
    assert_eq!(config.env, profile.config().env);
    assert!(!profile.directory().join("mimo-code-selected.sb").exists());
}

#[test]
fn acp_skill_coexistence_mimo_code_projects_only_validated_selection() {
    let workspace = tempfile::tempdir().unwrap();
    let selected = workspace.path().join("selected");
    let unselected = workspace.path().join("unselected");
    for (path, name) in [(&selected, "selected"), (&unselected, "unselected")] {
        fs::create_dir(path).unwrap();
        fs::write(
            path.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: fixture\n---\nRead a fixture."),
        )
        .unwrap();
    }
    let view = MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "selected".into(),
            path: selected.to_str().unwrap().into(),
        }],
        workspace.path(),
        "mimo-code-coexistence",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    assert!(super::super::validate("mimo-code", true).is_ok());
    let profile = profile();
    let base_policy = profile.write_config(POLICY_FILE, b"(deny file-write*)\n").unwrap();
    let mut config = profile.config().clone();
    config.env.insert(SANDBOX.into(), base_policy.to_string_lossy().into_owned());
    configure(&profile, &mut config, Some(&view)).unwrap();

    let root = view.root().canonicalize().unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&config.env[CONFIG_CONTENT]).unwrap(),
        json!({"skills": {"paths": [root]}})
    );
    assert_eq!(fs::read_to_string(base_policy).unwrap(), "(deny file-write*)\n");
    assert_eq!(
        config.env.get("VW_ACP_LAUNCH_REFERENCE"),
        profile.config().env.get("VW_ACP_LAUNCH_REFERENCE")
    );
    let policy = fs::read_to_string(&config.env[SANDBOX]).unwrap();
    assert!(policy.starts_with("(deny file-write*)\n"));
    assert!(policy.contains(selected.canonicalize().unwrap().to_str().unwrap()));
    assert!(!policy.contains(unselected.to_str().unwrap()));
    assert_eq!(policy.matches("(allow file-read-data").count(), 2);
}
