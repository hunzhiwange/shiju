use super::Input;
use desktop_provider_apps::AgentAppsContext;
pub struct Harness<S> { pub state: S, pub input: Input }
pub fn input() -> Input {
    Input {
        connection_error: String::new(),
        context: Some(AgentAppsContext { gateway: "http://localhost".into(), tenant: Default::default(), project: Some("/project".into()) }),
        project: Some("project".into()), directory: Some("/project".into()), host: "host".into(),
        catalog: 1, loopback: true, targets: vec![], repository_version: 0, job_active: false,
    }
}
