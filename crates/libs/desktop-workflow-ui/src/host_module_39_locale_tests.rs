use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_13_message_feedback_switches_language() {
    use crate::message::WorkflowMessage;
    let mut state = desktop_workflow_state::state::WorkflowState::default();
    let mut context = crate::controller::Context {
        workflow_state: &mut state,
        window_size: (1200.0, 800.0),
        tenant: vw_api_transport::TenantScope::new("10000", "test-company").unwrap(),
        gateway: || Err("isolated".into()),
        sync_top_tab: false,
    };
    let _ = crate::controller::update(
        &mut context,
        WorkflowMessage::DeleteSavedAppFinished(Ok("raw/%s/%n/中文".into())),
    );
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "已删除应用"),
        (DesktopLocale::TraditionalChinese, "已刪除應用"),
        (DesktopLocale::English, "Application deleted"),
    ] {
        assert_eq!(
            context.workflow_state.status_text(I18n::new(locale)).as_deref(),
            Some(expected)
        );
    }
    let raw = "服务错误 中文 %s %n";
    let _ = crate::controller::update(
        &mut context,
        WorkflowMessage::DeleteSavedAppFinished(Err(raw.to_string().into())),
    );
    assert_eq!(context.workflow_state.error_message.as_deref(), Some(raw));
}

#[test]
fn desktop_gettext_area_13_lifecycle_validation_keeps_template_and_identifier() {
    use crate::workflow_manager::{self, WorkflowManagerMessage};
    let mut state = workflow_manager::WorkflowManagerState::default();
    let mut registry = workflow_manager::registry_from_runtime();
    for message in [
        WorkflowManagerMessage::StartNew,
        WorkflowManagerMessage::DraftIdChanged("用户/%s/%n".into()),
        WorkflowManagerMessage::DraftLabelChanged(String::new()),
        WorkflowManagerMessage::SaveDraft,
    ] {
        let _ = workflow_manager::update(&mut state, &mut registry, message);
    }
    let message = state.feedback().unwrap().localized.as_ref().unwrap();
    assert_eq!(
        message.render(I18n::new(DesktopLocale::English)),
        "Custom workflow label is required: 用户/%s/%n"
    );
    assert_eq!(
        message.render(I18n::new(DesktopLocale::TraditionalChinese)),
        "自訂工作流 label 不能為空: 用户/%s/%n"
    );
}
