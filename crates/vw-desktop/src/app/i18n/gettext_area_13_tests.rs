use super::I18n;
use vw_config_types::ui::DesktopLocale;
#[test]
fn desktop_gettext_area_13_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "草稿待保存"),
        (DesktopLocale::TraditionalChinese, "草稿待儲存"),
        (DesktopLocale::English, "Unsaved draft"),
    ] {
        assert_eq!(I18n::new(locale).__("草稿待保存"), expected);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_13_message_feedback_switches_language() {
    use crate::apps::workflow::{self, WorkflowMessage};
    let (mut app, _) = crate::app::App::new();
    let _ = workflow::update(
        &mut app,
        WorkflowMessage::DeleteSavedAppFinished(Ok("raw/%s/%n/中文".into())),
    );
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "已删除应用"),
        (DesktopLocale::TraditionalChinese, "已刪除應用"),
        (DesktopLocale::English, "Application deleted"),
    ] {
        assert_eq!(app.workflow_state.status_text(I18n::new(locale)).as_deref(), Some(expected));
    }
    let raw = "服务错误 中文 %s %n";
    let _ = workflow::update(
        &mut app,
        WorkflowMessage::DeleteSavedAppFinished(Err(raw.to_string().into())),
    );
    assert_eq!(app.workflow_state.error_message.as_deref(), Some(raw));
}

#[test]
fn desktop_gettext_area_13_status_keeps_parameters_and_rejects_stale_metadata() {
    use super::format::Message;
    use crate::apps::workflow::state::WorkflowState;
    let mut state = WorkflowState::default();
    state.set_status(Message::new("已加载 %s", vec!["用户/%s/%n/中文".into()]));
    assert_eq!(
        state.status_text(I18n::new(DesktopLocale::English)).unwrap(),
        "Loaded 用户/%s/%n/中文"
    );
    assert_eq!(
        state.status_text(I18n::new(DesktopLocale::TraditionalChinese)).unwrap(),
        "已載入 用户/%s/%n/中文"
    );
    state.status_message = Some("raw 已加载 %s".into());
    assert_eq!(state.status_text(I18n::new(DesktopLocale::English)).unwrap(), "raw 已加载 %s");
}

#[test]
fn desktop_gettext_area_13_catalog_covers_display_calls_and_descriptors() {
    let en = super::catalog::parse_catalog(include_str!("locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("locales/zh-TW.json")).unwrap();
    let literal = regex::Regex::new(r#"(?:\.__|\.format|i18n::format::Message::new|workflow::Feedback::new)\(\s*("(?:[^"\\]|\\.)*")"#).unwrap();
    let check = |key: &str| {
        assert!(en.contains_key(key), "missing English: {key}");
        assert!(tw.contains_key(key), "missing Traditional Chinese: {key}");
    };
    for descriptor in crate::apps::workflow::model::supported_node_types() {
        if descriptor.label.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c)) {
            check(descriptor.label);
        }
        check(descriptor.summary);
    }
    for source in [
        include_str!("../views/workflow_manager/mod.rs"),
        include_str!("../../apps/workflow/canvas/handles.rs"),
        include_str!("../../apps/workflow/canvas/mod.rs"),
        include_str!("../../apps/workflow/canvas/render.rs"),
        include_str!("../../apps/workflow/canvas/utils.rs"),
        include_str!("../../apps/workflow/message/app.rs"),
        include_str!("../../apps/workflow/message/canvas.rs"),
        include_str!("../../apps/workflow/message/document.rs"),
        include_str!("../../apps/workflow/message/mod.rs"),
        include_str!("../../apps/workflow/message/node.rs"),
        include_str!("../../apps/workflow/message/run.rs"),
        include_str!("../../apps/workflow/mod.rs"),
        include_str!("../../apps/workflow/model/creation.rs"),
        include_str!("../../apps/workflow/model/defaults.rs"),
        include_str!("../../apps/workflow/model/dify.rs"),
        include_str!("../../apps/workflow/model/loader.rs"),
        include_str!("../../apps/workflow/model/mod.rs"),
        include_str!("../../apps/workflow/model/registry.rs"),
        include_str!("../../apps/workflow/model/save.rs"),
        include_str!("../../apps/workflow/state/app_ui.rs"),
        include_str!("../../apps/workflow/state/canvas_ops.rs"),
        include_str!("../../apps/workflow/state/core.rs"),
        include_str!("../../apps/workflow/state/draft_build.rs"),
        include_str!("../../apps/workflow/state/ids.rs"),
        include_str!("../../apps/workflow/state/if_else_helpers.rs"),
        include_str!("../../apps/workflow/state/mod.rs"),
        include_str!("../../apps/workflow/state/node_editor_branches.rs"),
        include_str!("../../apps/workflow/state/node_editor_integrations.rs"),
        include_str!("../../apps/workflow/state/node_editor_start.rs"),
        include_str!("../../apps/workflow/state/node_ops.rs"),
        include_str!("../../apps/workflow/state/node_validation.rs"),
        include_str!("../../apps/workflow/state/start_variable_drafts.rs"),
        include_str!("../../apps/workflow/state/start_variable_validation.rs"),
        include_str!("../../apps/workflow/state/variables.rs"),
        include_str!("../../apps/workflow/state/visual_sync.rs"),
        include_str!("../../apps/workflow/state/yaml_helpers.rs"),
        include_str!("../../apps/workflow/view/app_editor.rs"),
        include_str!("../../apps/workflow/view/app_list.rs"),
        include_str!("../../apps/workflow/view/canvas_shell.rs"),
        include_str!("../../apps/workflow/view/data_nodes.rs"),
        include_str!("../../apps/workflow/view/header.rs"),
        include_str!("../../apps/workflow/view/http_request.rs"),
        include_str!("../../apps/workflow/view/if_else.rs"),
        include_str!("../../apps/workflow/view/menu.rs"),
        include_str!("../../apps/workflow/view/mod.rs"),
        include_str!("../../apps/workflow/view/next_step.rs"),
        include_str!("../../apps/workflow/view/node_editor.rs"),
        include_str!("../../apps/workflow/view/node_editor_fields.rs"),
        include_str!("../../apps/workflow/view/node_visual.rs"),
        include_str!("../../apps/workflow/view/node_visual_integrations.rs"),
        include_str!("../../apps/workflow/view/run_preview.rs"),
        include_str!("../../apps/workflow/view/start_inputs.rs"),
        include_str!("../../apps/workflow/view/start_meta.rs"),
        include_str!("../../apps/workflow/view/start_panel.rs"),
        include_str!("../../apps/workflow/view/styles.rs"),
        include_str!("../../apps/workflow/view/toolbar.rs"),
        include_str!("../../apps/workflow/view/variables.rs"),
        include_str!("workflow.rs"),
    ] {
        for capture in literal.captures_iter(source) {
            let key: String = serde_json::from_str(&capture[1]).unwrap();
            if key.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c)) {
                check(&key);
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_13_save_failure_toast_keeps_nested_error_semantics() {
    use crate::apps::workflow::{self, WorkflowMessage};
    let (mut app, _) = crate::app::App::new();
    let error = super::workflow::Feedback::new("写入工作流文件失败: %s", vec!["用户/%s/%n".into()]);
    let _ = workflow::update(&mut app, WorkflowMessage::SaveActiveAppFinished(Err(error)));
    let message = app.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
    assert_eq!(
        message.render(I18n::new(DesktopLocale::English)).unwrap(),
        "Save failed\nFailed to write workflow file: 用户/%s/%n"
    );
    assert_eq!(
        message.render(I18n::new(DesktopLocale::TraditionalChinese)).unwrap(),
        "儲存失敗\n寫入工作流檔案失敗: 用户/%s/%n"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_13_locale_selection_refreshes_tab_without_navigation() {
    use crate::app::message::{GeneralSettingsMessage, SettingsMessage};
    let (mut app, _) = crate::app::App::new();
    crate::apps::workflow::sync_top_tab(&mut app);
    let active = app.active_tab_id.clone();
    app.desktop_locale_saving = true;
    let _ = crate::app::message::settings::update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::LocaleSelected(DesktopLocale::English)),
    );
    assert_eq!(app.active_tab_id, active);
    assert_eq!(
        app.open_tabs.iter().find(|tab| tab.id == "workflow_tool").unwrap().title,
        "Workflows"
    );
    app.workflow_state.active_app_id = Some("raw".into());
    app.workflow_state.source_name = "用户/%s/%n".into();
    crate::apps::workflow::refresh_top_tab_title(&mut app);
    assert_eq!(
        app.open_tabs.iter().find(|tab| tab.id == "workflow_tool").unwrap().title,
        "用户/%s/%n"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_13_lifecycle_validation_keeps_template_and_identifier() {
    use crate::app::views::workflow_manager::{self, WorkflowManagerMessage};
    let (mut app, _) = crate::app::App::new();
    for message in [
        WorkflowManagerMessage::StartNew,
        WorkflowManagerMessage::DraftIdChanged("用户/%s/%n".into()),
        WorkflowManagerMessage::DraftLabelChanged(String::new()),
        WorkflowManagerMessage::SaveDraft,
    ] {
        let _ = workflow_manager::update_app(&mut app, message);
    }
    let message = app.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
    assert_eq!(
        message.render(I18n::new(DesktopLocale::English)).unwrap(),
        "Custom workflow label is required: 用户/%s/%n"
    );
    assert_eq!(
        message.render(I18n::new(DesktopLocale::TraditionalChinese)).unwrap(),
        "自訂工作流 label 不能為空: 用户/%s/%n"
    );
}
