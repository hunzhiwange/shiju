use std::collections::HashMap;
struct LocalSkillsHost;
impl crate::admission_host::AdmissionHost for LocalSkillsHost {
    async fn config() -> anyhow::Result<agent_config::runtime::Config> { anyhow::bail!("local skills fixture has no request config") }
    fn project() -> Option<String> { None }
    async fn fork_selectable(_: &str, _: &str) -> anyhow::Result<bool> { anyhow::bail!("local skills fixture has no sessions") }
    async fn load_binding(_: &str) -> anyhow::Result<Option<crate::agent_binding::AgentBinding>> { anyhow::bail!("local skills fixture has no sessions") }
    async fn save_binding(_: &str, _: &str, _: &crate::agent_binding::AgentBinding) -> anyhow::Result<()> { anyhow::bail!("local skills fixture has no sessions") }
    async fn history_agents(_: &str) -> anyhow::Result<Vec<String>> { anyhow::bail!("local skills fixture has no sessions") }
    async fn agent(_: &str) -> Option<config_agent::AgentDefinitionConfig> { None }
    async fn default_model() -> anyhow::Result<config_agent_profile::AgentModelReference> { anyhow::bail!("local skills fixture has no default model") }
    fn skills(workspace:&std::path::Path, config:&agent_config::runtime::Config)->HashMap<String,bool> {
        agent_skills::catalog::load_skills_with_config(workspace,&config.skills,false, |_| Vec::new()).iter().map(|skill|(skill.invocation_name().to_string(),true)).collect()
    }
    async fn mcp(_: &std::path::Path,_: &config_agent_profile::AgentRuntime)->anyhow::Result<HashMap<String,bool>> { anyhow::bail!("local skill fixture has no MCP inventory") }
    async fn permission_ceiling(_: &str)->anyhow::Result<permission_runtime::next::Ruleset> { Ok(Default::default()) }
}
fn run_in_temporary_storage(test: &str) -> bool {
    const CHILD_TEST: &str = "SHIJU_HOST_MIGRATION_23_SKILLS_CHILD";
    if std::env::var(CHILD_TEST).as_deref() == Ok(test) {
        return false;
    }
    let root = tempfile::tempdir().expect("temporary storage root");
    let status = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .arg(format!("{}::{test}", module_path!().split_once("::").expect("crate module").1))
        .args(["--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_TEST, test)
        .env("VIBEWINDOW_TEST_ROOT", root.path())
        .env("VIBEWINDOW_TEST_HOME", root.path())
        .status()
        .expect("isolated test process");
    assert!(status.success(), "isolated test failed: {test}: {status}");
    true
}

#[test]
fn agent_native_defaults_do_not_inject_disabled_skills() {
    if run_in_temporary_storage("agent_native_defaults_do_not_inject_disabled_skills") { return; }
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let dir = workspace.path().join("skills/disabled");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("SKILL.md"),
        "---\nname: disabled\ndescription: Disabled fixture\n---\nMUST_NOT_INJECT\n",
    )
    .unwrap();
    std::fs::write(agent_skills::local::local_skill_disabled_marker_path(&dir), "").unwrap();
    let mut config = agent_config::runtime::Config::default();
    config.agents.insert(
        "main".into(),
        serde_json::from_value(serde_json::json!({
            "provider":"", "model":"", "defaults":{"skills":{"mode":"fixed","value":["disabled"]}}
        }))
        .unwrap(),
    );
    assert!(
        runtime
            .block_on(crate::capability_policy::native_agent_defaults::<LocalSkillsHost>(
                &config,
                "main",
                None,
                workspace.path()
            ))
            .is_err()
    );
}

#[test]
fn agent_config_cleanup_native_skill_inheritance_uses_allowed_source() {
    if run_in_temporary_storage("agent_config_cleanup_native_skill_inheritance_uses_allowed_source") { return; }
    use config_agent_profile::AgentDefault;
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let dir = workspace.path().join("skills/allowed");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SKILL.md"), "---\nname: allowed\ndescription: fixture\n---\n")
        .unwrap();
    let mut config = agent_config::runtime::Config::default();
    config.agents.insert(
        "main".into(),
        serde_json::from_value(
            serde_json::json!({"provider":"", "model":"", "allowed_skills":["allowed"], "skill_policy":"strict"}),
        )
        .unwrap(),
    );
    for (selection, expected) in [
        (AgentDefault::Inherit, vec!["allowed"]),
        (AgentDefault::Fixed(vec![]), vec![]),
        (AgentDefault::Fixed(vec!["allowed".into()]), vec!["allowed"]),
    ] {
        config.agents.get_mut("main").unwrap().defaults.skills = selection;
        let result = runtime
            .block_on(crate::capability_policy::native_agent_defaults::<LocalSkillsHost>(
                &config,
                "main",
                None,
                workspace.path(),
            ))
            .unwrap()
            .unwrap();
        assert_eq!(result.effective.skills, expected);
    }
    config.agents.get_mut("main").unwrap().defaults.skills =
        AgentDefault::Fixed(vec!["missing".into()]);
    assert!(
        runtime
            .block_on(crate::capability_policy::native_agent_defaults::<LocalSkillsHost>(
                &config,
                "main",
                None,
                workspace.path()
            ))
            .is_err()
    );
}
