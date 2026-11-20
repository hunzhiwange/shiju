use super::{append_skill_root, configure};
use agent_config::schema::AcpAgentConfig;

#[test]
fn acp_skill_coexistence_grok_appends_only_the_selected_root() {
    let original = r#"
[skills]
paths = ["/native"]
ignore = ["/ignored"]
disabled = ["disabled"]

[models]
default = "shiju-session"
"#;
    let root = std::path::Path::new("/selected");
    let mounted = append_skill_root(original, root).unwrap();
    let config = mounted.parse::<toml::Table>().unwrap();
    let mut expected = original.parse::<toml::Table>().unwrap();
    expected["skills"]["paths"].as_array_mut().unwrap().push("/selected".into());
    assert_eq!(config, expected);
    assert_eq!(append_skill_root(&mounted, root).unwrap(), mounted);
}

#[test]
fn acp_skill_coexistence_grok_without_selection_preserves_launch() {
    let mut config = AcpAgentConfig {
        command: "grok".into(),
        args: vec!["agent".into(), "stdio".into()],
        ..Default::default()
    };
    configure(&mut config, None).unwrap();
    assert_eq!(config.command, "grok");
    assert_eq!(config.args, ["agent", "stdio"]);
    assert!(config.env.is_empty());
}
