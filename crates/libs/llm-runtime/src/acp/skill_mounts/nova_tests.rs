use super::*;
use std::{cell::Cell, path::Path};

#[derive(Clone)]
struct TestView;

thread_local! {
    static VALIDATIONS: Cell<bool> = const { Cell::new(false) };
}

impl SkillLease for TestView {
    fn fingerprint(&self) -> &str {
        "fixture"
    }

    fn root(&self) -> &Path {
        Path::new("/private/fixture/skills")
    }

    fn validate(&self) -> Result<(), Error> {
        VALIDATIONS.with(|validated| validated.set(true));
        Ok(())
    }
}

#[test]
fn acp_skill_coexistence_nova_gates_only_selected_skills() {
    let mut config = AcpAgentConfig {
        command: "/private/fixture/unreviewed-nova".into(),
        args: vec!["acp".into()],
        ..Default::default()
    };

    assert!(super::super::validate("nova", false).is_ok());
    assert_eq!(
        super::super::validate("nova", true).is_ok(),
        cfg!(all(target_os = "macos", target_arch = "aarch64"))
    );
    assert!(configure(&config, None::<&TestView>).is_ok());
    assert!(configure(&config, Some(&TestView)).is_err());
    VALIDATIONS.with(|validated| assert!(!validated.get()));

    config.env.insert("VW_ACP_LAUNCH_REFERENCE".into(), "fixture".into());
    assert!(configure(&config, Some(&TestView)).is_err());
    VALIDATIONS.with(|validated| {
        assert_eq!(validated.get(), supports_session_view_target());
    });
    assert!(configure(&config, None::<&TestView>).is_ok());
}
