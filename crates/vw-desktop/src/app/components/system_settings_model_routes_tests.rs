use super::*;
use crate::app::App;
use crate::app::state::ModelRoute;
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
    let tree = element_tree(field_row("模型", "命中后优先使用的模型。", text("gpt-5")));

    assert_eq!(tree.children.len(), 2);
    assert_eq!(tree.children[0].children.len(), 2);
    assert!(tree.children[1].children.is_empty());
}

#[test]
fn view_empty_routes_adds_empty_panel_after_header_and_section() {
    let mut app = test_app();
    app.model_routes_settings.routes.clear();

    let tree = element_tree(view(&app));

    assert_eq!(tree.children.len(), 3);
    assert_eq!(tree.children[0].children.len(), 2);
    assert_eq!(tree.children[2].children.len(), 1);
}

#[test]
fn view_route_cards_replace_empty_panel_and_render_four_fields_per_route() {
    let mut populated = test_app();
    populated.model_routes_settings.routes = vec![
        ModelRoute {
            pattern: "reasoning".to_string(),
            provider: "openai".to_string(),
            model: "gpt-5".to_string(),
            priority_input: "10".to_string(),
        },
        ModelRoute {
            pattern: "fast".to_string(),
            provider: "anthropic".to_string(),
            model: "claude".to_string(),
            priority_input: "1".to_string(),
        },
    ];

    let tree = element_tree(view(&populated));

    assert_eq!(tree.children.len(), 4);
    for card in &tree.children[2..] {
        assert_eq!(card.children.len(), 2);
        assert_eq!(card.children[0].children.len(), 3);

        let fields = &card.children[1];
        assert_eq!(fields.children.len(), 4);
        for field in &fields.children {
            assert_eq!(field.children.len(), 2);
        }
    }
}

#[test]
fn view_route_with_empty_values_keeps_all_editors_available() {
    let mut app = test_app();
    app.model_routes_settings.routes = vec![ModelRoute {
        pattern: String::new(),
        provider: String::new(),
        model: String::new(),
        priority_input: String::new(),
    }];

    let tree = element_tree(view(&app));
    let fields = &tree.children[2].children[1];

    assert_eq!(tree.children.len(), 3);
    assert_eq!(fields.children.len(), 4);
}

#[test]
fn source_wires_each_editor_action_to_model_routes_message() {
    let source = include_str!("system_settings_model_routes.rs");

    assert!(source.contains("ModelRoutesMessage::AddRoute"));
    assert!(source.contains("ModelRoutesMessage::RemoveRoute(idx)"));
    assert!(source.contains("ModelRoutesMessage::PatternChanged(idx, value)"));
    assert!(source.contains("ModelRoutesMessage::ProviderChanged(idx, value)"));
    assert!(source.contains("ModelRoutesMessage::ModelChanged("));
    assert!(source.contains("ModelRoutesMessage::PriorityChanged(idx, value)"));
}
