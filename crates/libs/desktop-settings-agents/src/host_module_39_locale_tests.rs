use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_7_acp_validation_is_localized_without_changing_input() {
    let mut state = crate::acp_state::AcpSettingsState::default();
    state.new_agent_name_input.clear();
    let mut agents = Vec::new();
    let mut agent = None;
    let mut input = crate::acp_update::Update {
        state: &mut state,
        acp_agents: &mut agents,
        acp_agent: &mut agent,
        access: desktop_config::ConfigAccess::new(
            gateway_endpoint::GatewayEndpoint::default(),
            None,
        ),
        client: Err("isolated".into()),
        snapshot_applied: false,
        toasts: Vec::new(),
    };
    let _ = crate::acp_update::update(&mut input, crate::acp_update_message::AcpMessage::AddAgent);
    let status = state.error_translation.as_ref().unwrap();
    assert_eq!(
        status.render(I18n::new(DesktopLocale::English)).unwrap(),
        "ACP name cannot be empty"
    );
    assert_eq!(
        status.render(I18n::new(DesktopLocale::TraditionalChinese)).unwrap(),
        "ACP 名稱不得為空"
    );
    assert!(state.new_agent_name_input.is_empty());
}

#[test]
fn desktop_gettext_area_7_dialogue_feedback_switches_locale() {
    let mut persistence = desktop_config::config_desktop::DesktopPreferencesPersistence::default();
    let mut editor = iced::widget::text_editor::Content::new();
    let mut reasoning = true;
    let mut shell = false;
    let mut edit = false;
    let mut translation = None;
    let mut saved = None;
    let mut input = crate::dialogue_flow_update::Update {
        endpoint: gateway_endpoint::GatewayEndpoint::default(),
        project_path: None,
        desktop_preferences_persistence: &mut persistence,
        dialogue_flow_permission_editor: &mut editor,
        dialogue_flow_show_reasoning_summary: &mut reasoning,
        dialogue_flow_expand_shell_tool_section: &mut shell,
        dialogue_flow_expand_edit_tool_section: &mut edit,
        dialogue_flow_settings_translation: &mut translation,
        dialogue_flow_settings_save_message: &mut saved,
        patches: Vec::new(),
    };
    let raw = "原始 %n/%s 错误";
    let _ = crate::dialogue_flow_update::update(
        &mut input,
        crate::dialogue_flow_update::Message::DialogueFlowUiSettingsSaved(Err(raw.into())),
    );
    let status = input.dialogue_flow_settings_translation.as_ref().unwrap();
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        assert!(status.render(I18n::new(locale)).unwrap().ends_with(raw));
    }
    let _ = crate::dialogue_flow_update::update(
        &mut input,
        crate::dialogue_flow_update::Message::DialogueFlowUiSettingsSaved(Ok(())),
    );
    assert_eq!(
        input
            .dialogue_flow_settings_translation
            .as_ref()
            .unwrap()
            .render(I18n::new(DesktopLocale::English))
            .unwrap(),
        "Conversation flow configuration saved"
    );
}
