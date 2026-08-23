//! 验证查询分类规则卡片、模型路由选择和编辑消息连线。
//!
//! 规则夹具覆盖空列表、单条和模型路由，源码连线检查用于防止编辑消息遗漏。

use super::*;
use crate::app::state::{ModelRoute, QueryClassificationRuleInput};
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::text;

fn test_app() -> App {
    App::new().0
}

fn element_tree(element: Element<'_, Message>) -> Tree {
    Tree::new(element.as_widget())
}

#[test]
fn field_row_builds_label_description_and_control_structure() {
    let tree = element_tree(field_row("pattern", "匹配关键字或模式。", text("bug")));

    assert_eq!(tree.children.len(), 2);
    assert_eq!(tree.children[0].children.len(), 2);
    assert!(tree.children[1].children.is_empty());
}

#[test]
fn view_empty_rules_adds_empty_panel_after_base_sections() {
    let mut app = test_app();
    app.query_classification_settings.rules.clear();

    let tree = element_tree(view(&app));

    assert_eq!(tree.children.len(), 5);
    assert_eq!(tree.children[0].children.len(), 3);
    assert_eq!(tree.children[2].children.len(), 1);
    assert_eq!(tree.children[4].children.len(), 1);
}

#[test]
fn view_rule_cards_replace_empty_panel_and_render_three_fields_per_rule() {
    let mut app = test_app();
    app.query_classification_settings.enabled = true;
    app.query_classification_settings.rules = vec![
        QueryClassificationRuleInput {
            pattern: "bug".to_string(),
            category: "reasoning".to_string(),
            priority_input: "10".to_string(),
        },
        QueryClassificationRuleInput {
            pattern: "test".to_string(),
            category: "fast".to_string(),
            priority_input: "-1".to_string(),
        },
    ];

    let tree = element_tree(view(&app));

    assert_eq!(tree.children.len(), 6);
    for card in &tree.children[4..] {
        assert_eq!(card.children.len(), 2);
        assert_eq!(card.children[0].children.len(), 3);

        let fields = &card.children[1];
        assert_eq!(fields.children.len(), 3);
        for field in &fields.children {
            assert_eq!(field.children.len(), 2);
        }
    }
}

#[test]
fn view_single_rule_uses_one_card_without_empty_state() {
    let mut app = test_app();
    app.query_classification_settings.rules = vec![QueryClassificationRuleInput {
        pattern: "部署".to_string(),
        category: "operations".to_string(),
        priority_input: String::new(),
    }];

    let tree = element_tree(view(&app));

    assert_eq!(tree.children.len(), 5);
    let card_content = &tree.children[4];
    assert_eq!(card_content.children.len(), 2);
    assert_eq!(card_content.children[1].children.len(), 3);
}

#[test]
fn view_renders_category_picker_for_configured_model_routes() {
    let mut app = test_app();
    app.model_routes_settings.routes = vec![ModelRoute {
        pattern: "code".to_string(),
        provider: "openai".to_string(),
        model: "gpt-5".to_string(),
        priority_input: "100".to_string(),
    }];
    app.query_classification_settings.rules = vec![QueryClassificationRuleInput {
        pattern: "写代码".to_string(),
        category: "code".to_string(),
        priority_input: "100".to_string(),
    }];

    let tree = element_tree(view(&app));

    assert_eq!(tree.children.len(), 5);
}

#[test]
fn source_wires_each_editor_action_to_query_classification_message() {
    let source = include_str!("system_settings_query_classification.rs");

    assert!(source.contains("QueryClassificationMessage::AddRule"));
    assert!(source.contains("QueryClassificationMessage::EnabledToggled(value)"));
    assert!(source.contains("QueryClassificationMessage::RemoveRule(idx)"));
    assert!(source.contains("QueryClassificationMessage::PatternChanged(idx, value)"));
    assert!(source.contains("QueryClassificationMessage::CategoryChanged(idx, value)"));
    assert!(source.contains("QueryClassificationMessage::PriorityChanged(idx, value)"));
    assert!(source.contains("pick_list"));
    assert!(source.contains("ModelConfigTabMessage::Selected(ModelConfigTab::ModelRoutes)"));
}
