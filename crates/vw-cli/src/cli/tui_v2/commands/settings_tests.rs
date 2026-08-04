use super::super::model::settings_panel::{SettingsPanel, SettingsValues};
use super::super::model::{PermissionMode, ReasoningEffort, SessionCapabilityMatrix};
use super::super::state::TuiState;
use super::settings::{SettingsCommand, SettingsRuntime, apply_settings_command, commit_settings};

#[derive(Default)]
struct RecordingSettings {
    saved: Vec<SettingsValues>,
    fail: bool,
}

impl SettingsRuntime for RecordingSettings {
    fn save(&mut self, values: &SettingsValues) -> Result<(), String> {
        if self.fail {
            return Err("config patch failed".to_string());
        }
        self.saved.push(values.clone());
        Ok(())
    }
}

#[test]
fn tui_settings_commands_preview_commit_and_rollback_exactly() {
    let original = SettingsValues::default();
    let mut panel = SettingsPanel::new(original.clone());
    panel.query = "主题".to_string();
    assert_eq!(panel.visible_rows().len(), 1);
    panel.preview_theme("Light").unwrap();
    assert_eq!(panel.pending.theme, "Light");
    assert_eq!(panel.original.theme, "Dark");

    let mut runtime = RecordingSettings { fail: true, ..RecordingSettings::default() };
    assert!(commit_settings(&mut panel, &mut runtime).is_err());
    assert_eq!(*panel.pending, original, "failed save rolls preview back");

    panel.preview_theme("DarkDaltonized").unwrap();
    runtime.fail = false;
    commit_settings(&mut panel, &mut runtime).unwrap();
    assert_eq!(panel.original.theme, "DarkDaltonized");
    assert_eq!(runtime.saved[0].theme, "DarkDaltonized");

    panel.preview_theme("LightAnsi").unwrap();
    panel.cancel();
    assert_eq!(panel.pending.theme, "DarkDaltonized");
    assert!(panel.preview_theme("imaginary").unwrap_err().contains("unsupported"));
    assert!(
        panel
            .rows
            .iter()
            .find(|row| row.label == "权限模式")
            .unwrap()
            .unavailable_reason
            .unwrap()
            .contains("专用确认")
    );
}

#[test]
fn settings_commands_validate_capabilities_persist_supported_fields_and_rollback() {
    let state = TuiState::default();
    for command in [
        "settings",
        "config",
        "model",
        "acp",
        "effort",
        "fast",
        "theme",
        "plan",
        "permissions",
        "allowed-tools",
    ] {
        assert!(state.command_registry.resolve(command).is_some(), "/{command} must be registered");
    }

    let capabilities = SessionCapabilityMatrix {
        models: vec!["openai/gpt-5.6".to_string()],
        efforts: vec![ReasoningEffort::Low, ReasoningEffort::High],
        supports_thinking: false,
        supports_fast: false,
        permission_modes: vec![PermissionMode::Default, PermissionMode::Plan],
        delegate_agents: vec![super::super::model::DelegateAgentOption::main()],
        acp_backends: vec![super::super::model::AcpBackendOption::default_backend()],
    };
    let original = SettingsValues::default();
    let mut panel = SettingsPanel::new(original.clone());
    let mut runtime = RecordingSettings::default();

    apply_settings_command(
        &mut panel,
        &SettingsCommand::PermissionMode(PermissionMode::Plan),
        &capabilities,
        false,
        &mut runtime,
    )
    .unwrap();
    assert_eq!(panel.original.permission_mode, PermissionMode::Plan);

    runtime.fail = true;
    let before_failure = panel.original.clone();
    assert!(
        apply_settings_command(
            &mut panel,
            &SettingsCommand::Effort(ReasoningEffort::High),
            &capabilities,
            false,
            &mut runtime,
        )
        .is_err()
    );
    assert_eq!(panel.original, before_failure);
    assert_eq!(panel.pending, before_failure);
}

#[test]
fn settings_model_picker_commits_selected_catalog_model() {
    let capabilities = SessionCapabilityMatrix {
        models: vec!["openai/gpt-5.6".to_string(), "anthropic/claude-sonnet".to_string()],
        ..SessionCapabilityMatrix::default()
    };
    let mut original = SettingsValues::default();
    original.model = Some("openai/gpt-5.6".to_string());
    let mut panel = SettingsPanel::for_capabilities(original, &capabilities);

    assert!(panel.open_model_picker());
    panel.move_model_picker_selection(1);
    assert!(panel.confirm_model_picker());
    assert_eq!(panel.pending.model.as_deref(), Some("anthropic/claude-sonnet"));

    let mut runtime = RecordingSettings::default();
    commit_settings(&mut panel, &mut runtime).expect("selected model persists");
    assert_eq!(runtime.saved.len(), 1);
    assert_eq!(runtime.saved[0].model.as_deref(), Some("anthropic/claude-sonnet"));
    assert_eq!(panel.original.model.as_deref(), Some("anthropic/claude-sonnet"));
}
