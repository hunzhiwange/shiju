// 验证查询分类规则的规范化、校验与本地编辑状态。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use crate::app::state::QueryClassificationRuleInput;

fn app() -> App {
    App::new().0
}

#[test]
fn refresh_failure_preserves_draft_and_reports_error() {
    let mut app = app();
    app.query_classification_settings.enabled = true;
    app.query_classification_settings.rules = vec![QueryClassificationRuleInput {
        pattern: "keep".into(),
        category: "keep".into(),
        priority_input: "9".into(),
    }];
    let _ =
        update(&mut app, SettingsMessage::QueryClassification(QueryClassificationMessage::Refresh));
    let revision = app.model_routes_settings.persistence.revision;
    let _ = super::super::model_routes::update(
        &mut app,
        SettingsMessage::ModelRoutes(super::super::messages::ModelRoutesMessage::Loaded(
            revision,
            Err("offline".into()),
        )),
    );
    assert!(app.query_classification_settings.enabled);
    assert_eq!(app.query_classification_settings.rules[0].category, "keep");
    assert!(app.query_classification_settings.save_error.is_some());
    let _ = update(
        &mut app,
        SettingsMessage::QueryClassification(QueryClassificationMessage::CategoryChanged(
            0,
            "new".into(),
        )),
    );
    let _ = super::super::model_routes::update(
        &mut app,
        SettingsMessage::ModelRoutes(super::super::messages::ModelRoutesMessage::Loaded(
            revision,
            Ok((vec![], Default::default())),
        )),
    );
    assert_eq!(app.query_classification_settings.rules[0].category, "new");
}

#[test]
fn update_ignores_messages_for_other_settings_sections() {
    let mut app = app();
    app.query_classification_settings.enabled = true;
    app.query_classification_settings.rules = vec![QueryClassificationRuleInput::default()];
    app.query_classification_settings.save_error = Some("keep".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(app.query_classification_settings.enabled);
    assert_eq!(app.query_classification_settings.rules.len(), 1);
    assert_eq!(app.query_classification_settings.save_error.as_deref(), Some("keep"));
}
