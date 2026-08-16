use super::*;
use crate::app::state::EmbeddingRouteDraft;
use crate::app::{App, Message};
use iced::Element;
use iced::advanced::widget::Tree;

fn test_app() -> App {
    let mut app = App::new().0;
    app.embedding_routes_settings.routes.clear();
    app.embedding_routes_settings.save_error = None;
    app.embedding_routes_settings.save_success = false;
    app
}

fn element_tree(element: Element<'_, Message>) -> Tree {
    Tree::new(element.as_widget())
}

fn content_tree(tree: &Tree) -> &Tree {
    tree
}

#[test]
fn field_rows_build_label_description_and_input_structure() {
    let plain = element_tree(field_row("匹配模式", "说明", "semantic", "memory", |value| {
        Message::Settings(message::SettingsMessage::EmbeddingRoutes(
            EmbeddingRoutesMessage::PatternChanged(0, value),
        ))
    }));
    let secure = element_tree(secure_field_row("API Key", "说明", "KEY", "secret", |value| {
        Message::Settings(message::SettingsMessage::EmbeddingRoutes(
            EmbeddingRoutesMessage::ApiKeyChanged(0, value),
        ))
    }));

    for tree in [&plain, &secure] {
        let row = content_tree(tree);
        assert_eq!(row.children.len(), 2);
        assert_eq!(row.children[0].children.len(), 2);
        assert!(row.children[1].children.is_empty());
    }
}

#[test]
fn template_button_builds_two_line_action_content() {
    let tree = element_tree(template_button(
        "阿里 DashScope",
        "alibaba-cn / text-embedding-v4 / 1024",
        EmbeddingRoutesMessage::InsertAlibabaTemplate,
    ));

    assert_eq!(tree.children.len(), 1);
    assert_eq!(tree.children[0].children.len(), 2);
}

#[test]
fn embedded_view_empty_routes_shows_empty_panel_between_section_and_save_action() {
    let app = test_app();
    let tree = element_tree(embedded_view(&app));
    let content = content_tree(&tree);

    assert_eq!(content.children.len(), 5);
    assert_eq!(content.children[0].children.len(), 2);
    assert_eq!(content.children[4].children.len(), 2);
}

#[test]
fn embedded_view_success_banner_requires_success_without_error() {
    let mut app = test_app();
    app.embedding_routes_settings.save_success = true;
    let success_tree = element_tree(embedded_view(&app));
    assert_eq!(content_tree(&success_tree).children.len(), 6);

    app.embedding_routes_settings.save_error = Some("保存失败".to_string());
    let error_tree = element_tree(embedded_view(&app));
    assert_eq!(content_tree(&error_tree).children.len(), 6);
}

#[test]
fn embedded_view_route_cards_replace_empty_state_and_keep_one_card_per_route() {
    let mut app = test_app();
    app.embedding_routes_settings.routes = vec![
        EmbeddingRouteDraft {
            pattern: "memory".to_string(),
            provider: "alibaba-cn".to_string(),
            model: "text-embedding-v4".to_string(),
            dimensions: "1024".to_string(),
            api_key_input: "test-key".to_string(),
        },
        EmbeddingRouteDraft {
            pattern: "code".to_string(),
            provider: "openai".to_string(),
            model: "text-embedding-3-large".to_string(),
            dimensions: String::new(),
            api_key_input: String::new(),
        },
    ];

    let tree = element_tree(embedded_view(&app));
    let content = content_tree(&tree);

    assert_eq!(content.children.len(), 6);
    for card in &content.children[3..5] {
        assert_eq!(card.children.len(), 2);
        assert_eq!(card.children[0].children.len(), 4);
        assert_eq!(card.children[1].children.len(), 5);
    }
    assert_eq!(content.children[5].children.len(), 2);
}

#[test]
fn embedded_route_card_contains_five_complete_field_rows() {
    let mut app = test_app();
    app.embedding_routes_settings.routes = vec![EmbeddingRouteDraft {
        pattern: "memory".to_string(),
        provider: "alibaba-cn".to_string(),
        model: "text-embedding-v4".to_string(),
        dimensions: "1024".to_string(),
        api_key_input: "secret".to_string(),
    }];

    let tree = element_tree(embedded_view(&app));
    let card = &content_tree(&tree).children[3];
    let fields = &card.children[1];

    assert_eq!(fields.children.len(), 5);
    for field in &fields.children {
        let row = content_tree(field);
        assert_eq!(row.children.len(), 2);
        assert_eq!(row.children[0].children.len(), 2);
    }
}
