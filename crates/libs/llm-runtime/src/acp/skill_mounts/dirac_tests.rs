use super::*;
use std::path::Path;

#[derive(Clone)]
struct TestView;

impl SkillLease for TestView {
    fn fingerprint(&self) -> &str {
        "fixture"
    }

    fn root(&self) -> &Path {
        Path::new("/private/fixture/skills")
    }

    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}

#[test]
fn acp_skill_coexistence_dirac_requires_reviewed_launch_only_with_selection() {
    let mut config = AcpAgentConfig {
        command: "/private/fixture/unreviewed-dirac".to_string(),
        args: vec!["--acp".to_string()],
        ..Default::default()
    };

    assert!(configure(&config, None::<&TestView>).is_ok());
    assert!(configure(&config, Some(&TestView)).is_err());
    config.env.insert("VW_ACP_LAUNCH_REFERENCE".into(), "fixture".into());
    assert!(configure(&config, Some(&TestView)).is_err());
    assert!(configure(&config, None::<&TestView>).is_ok());
    assert_eq!(config.args, ["--acp"]);
}
