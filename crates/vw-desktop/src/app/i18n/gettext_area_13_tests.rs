use super::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_13_catalog_covers_display_calls_and_descriptors() {
    let en = super::catalog::parse_catalog(include_str!(
        "../../../../libs/desktop-i18n/src/locales/en.json"
    ))
    .unwrap();
    let tw = super::catalog::parse_catalog(include_str!(
        "../../../../libs/desktop-i18n/src/locales/zh-TW.json"
    ))
    .unwrap();
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
        include_str!("../../../../libs/desktop-workflow-canvas/src/canvas/handles.rs"),
        include_str!("../../apps/workflow/canvas/mod.rs"),
        include_str!("../../../../libs/desktop-workflow-canvas/src/canvas/render.rs"),
        include_str!("../../../../libs/desktop-workflow-canvas/src/canvas/utils.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/message/app.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/message/canvas.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/message/document.rs"),
        include_str!("../../apps/workflow/message/mod.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/message/node.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/message/run.rs"),
        include_str!("../../apps/workflow/mod.rs"),
        include_str!("../../../../libs/desktop-workflow-model/src/model/creation.rs"),
        include_str!("../../../../libs/desktop-workflow-model/src/model/defaults.rs"),
        include_str!("../../../../libs/desktop-workflow-model/src/model/dify.rs"),
        include_str!("../../../../libs/desktop-workflow-model/src/model/loader.rs"),
        include_str!("../../apps/workflow/model/mod.rs"),
        include_str!("../../../../libs/desktop-workflow-model/src/model/registry.rs"),
        include_str!("../../../../libs/desktop-workflow-model/src/model/save.rs"),
        include_str!("../../../../libs/desktop-workflow-state/src/state/app_ui.rs"),
        include_str!("../../../../libs/desktop-workflow-state/src/state/canvas_ops.rs"),
        include_str!("../../../../libs/desktop-workflow-state/src/state/core.rs"),
        include_str!("../../../../libs/desktop-workflow-state/src/state/draft_build.rs"),
        include_str!("../../../../libs/desktop-workflow-state/src/state/ids.rs"),
        include_str!("../../../../libs/desktop-workflow-state/src/state/if_else_helpers.rs"),
        include_str!("../../apps/workflow/state/mod.rs"),
        include_str!("../../../../libs/desktop-workflow-state/src/state/node_editor_branches.rs"),
        include_str!(
            "../../../../libs/desktop-workflow-state/src/state/node_editor_integrations.rs"
        ),
        include_str!("../../../../libs/desktop-workflow-state/src/state/node_editor_start.rs"),
        include_str!("../../../../libs/desktop-workflow-state/src/state/node_ops.rs"),
        include_str!("../../../../libs/desktop-workflow-state/src/state/node_validation.rs"),
        include_str!("../../../../libs/desktop-workflow-state/src/state/start_variable_drafts.rs"),
        include_str!(
            "../../../../libs/desktop-workflow-state/src/state/start_variable_validation.rs"
        ),
        include_str!("../../../../libs/desktop-workflow-state/src/state/variables.rs"),
        include_str!("../../../../libs/desktop-workflow-state/src/state/visual_sync.rs"),
        include_str!(
            "../../../../libs/desktop-workflow-state/src/state/visual_sync/input_nodes.rs"
        ),
        include_str!("../../../../libs/desktop-workflow-state/src/state/visual_sync/data_nodes.rs"),
        include_str!(
            "../../../../libs/desktop-workflow-state/src/state/visual_sync/integration_nodes.rs"
        ),
        include_str!("../../../../libs/desktop-workflow-state/src/state/yaml_helpers.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/app_editor.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/app_list.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/canvas_shell.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/data_nodes.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/header.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/http_request.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/if_else.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/menu.rs"),
        include_str!("../../apps/workflow/view/mod.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/next_step.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/node_editor.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/node_editor_fields.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/node_visual.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/node_visual_integrations.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/run_preview.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/start_inputs.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/start_meta.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/start_panel.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/styles.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/toolbar.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/view/variables.rs"),
        include_str!("../../../../libs/desktop-i18n/src/workflow.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/app_editor.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/app_list.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/canvas_shell.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/data_nodes.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/http_request.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/if_else.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/menu.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/mod.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/next_step.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/node_editor.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/node_editor_fields.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/node_visual.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/node_visual_integrations.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/run_preview.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/start_inputs.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/start_meta.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/start_panel.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/styles.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/tests.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/toolbar.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/variables.rs"),
        include_str!("../../../../libs/desktop-workflow-ui/src/page/video_edit.rs"),
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
    let message =
        app.toast_state.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
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
    let active = app.navigation_tabs.active_tab_id.clone();
    app.general_settings.desktop_locale_saving = true;
    let _ = crate::app::message::settings::update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::LocaleSelected(DesktopLocale::English)),
    );
    assert_eq!(app.navigation_tabs.active_tab_id, active);
    assert_eq!(
        app.navigation_tabs.open_tabs.iter().find(|tab| tab.id == "workflow_tool").unwrap().title,
        "Workflows"
    );
    app.workflow_state.active_app_id = Some("raw".into());
    app.workflow_state.source_name = "用户/%s/%n".into();
    crate::apps::workflow::refresh_top_tab_title(&mut app);
    assert_eq!(
        app.navigation_tabs.open_tabs.iter().find(|tab| tab.id == "workflow_tool").unwrap().title,
        "用户/%s/%n"
    );
}
