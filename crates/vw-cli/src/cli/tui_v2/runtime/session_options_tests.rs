use super::super::model::session_mode::ReasoningEffort;
use super::super::model::{PermissionMode, SessionCapabilityMatrix, SessionOptions};
use super::session_options::{SessionOptionsRuntime, commit_session_options};

#[derive(Default)]
struct RecordingRuntime {
    saved: Vec<SessionOptions>,
    error: Option<String>,
}

impl SessionOptionsRuntime for RecordingRuntime {
    fn save_session_options(&mut self, options: &SessionOptions) -> Result<(), String> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        self.saved.push(options.clone());
        Ok(())
    }
}

#[test]
fn tui_session_options_match_reference_picker_and_failure_semantics() {
    let capabilities = SessionCapabilityMatrix {
        models: vec!["openai/gpt-5.6".to_string()],
        efforts: vec![ReasoningEffort::Low, ReasoningEffort::High],
        supports_thinking: true,
        supports_fast: false,
        permission_modes: vec![
            PermissionMode::Default,
            PermissionMode::Plan,
            PermissionMode::Bypass,
        ],
        delegate_agents: vec![super::super::model::DelegateAgentOption::main()],
        acp_backends: vec![super::super::model::AcpBackendOption::default_backend()],
    };
    let mut current = SessionOptions::default();
    assert_eq!(current.cycle_permission_mode(&capabilities), PermissionMode::Plan);

    let requested = SessionOptions {
        model: Some("openai/gpt-5.6".to_string()),
        effort: ReasoningEffort::High,
        thinking: true,
        permission_mode: PermissionMode::Plan,
        ..SessionOptions::default()
    };
    let mut runtime = RecordingRuntime::default();
    commit_session_options(&mut current, requested.clone(), &capabilities, false, &mut runtime)
        .unwrap();
    assert_eq!(current, requested);

    runtime.error = Some("gateway rejected options".to_string());
    let before_failure = current.clone();
    let mut failed = current.clone();
    failed.effort = ReasoningEffort::Low;
    assert!(
        commit_session_options(&mut current, failed, &capabilities, false, &mut runtime,).is_err()
    );
    assert_eq!(current, before_failure, "UI state only commits after runtime success");

    runtime.error = None;
    let mut bypass = current.clone();
    bypass.permission_mode = PermissionMode::Bypass;
    assert!(
        commit_session_options(&mut current, bypass.clone(), &capabilities, false, &mut runtime,)
            .unwrap_err()
            .contains("explicit confirmation")
    );
    commit_session_options(&mut current, bypass, &capabilities, true, &mut runtime).unwrap();
    assert!(current.bypass_confirmed);

    let mut unsupported = current.clone();
    unsupported.fast = true;
    assert!(
        commit_session_options(&mut current, unsupported, &capabilities, true, &mut runtime,)
            .unwrap_err()
            .contains("not supported")
    );
}
