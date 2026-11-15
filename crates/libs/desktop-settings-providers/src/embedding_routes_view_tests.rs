//! 验证嵌入路由设置的空态、路由卡片、字段结构与保存反馈。
//! 多路由状态要求每个配置独立保留完整字段，成功横幅不得掩盖错误。

use super::*;
use crate::app_init::state::EmbeddingRouteDraft;
use crate::view_message::{Message, SettingsMessage};
use iced::Element;
use iced::advanced::widget::Tree;

// 路由用例共享最小应用状态，以便精确比较空态与路由卡片元素树。
struct Fixture { embedding_routes_settings: EmbeddingRoutesSettingsState }
fn test_app() -> Fixture { Fixture { embedding_routes_settings: EmbeddingRoutesSettingsState::default() } }
fn style() -> ViewStyle<'static> {
    fn help<'a>(_: &'a str, value: std::borrow::Cow<'a,str>, _: Option<iced::Color>, _: desktop_i18n::I18n) -> Element<'a,Message> { text(value).into() }
    static THEME: std::sync::LazyLock<iced::Theme> = std::sync::LazyLock::new(|| iced::Theme::Light);
    ViewStyle { locale: config_ui::DesktopLocale::SimplifiedChinese, theme: &THEME, help_text: help }
}

fn element_tree(element: Element<'_, Message>) -> Tree {
    Tree::new(element.as_widget())
}

fn content_tree(tree: &Tree) -> &Tree {
    tree
}

#[test]
fn field_rows_build_label_description_and_input_structure() {
    let app = test_app();
    let plain =
        element_tree(field_row(style(), "匹配模式", "说明", "semantic", "memory", |value| {
            Message::Settings(SettingsMessage::EmbeddingRoutes(
                EmbeddingRoutesMessage::PatternChanged(0, value),
            ))
        }));
    let secure =
        element_tree(secure_field_row(style(), "API Key", "说明", "KEY", "secret", |value| {
            Message::Settings(SettingsMessage::EmbeddingRoutes(
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
fn template_button_builds_name_only_action_content() {
    let tree = element_tree(template_button(
        "阿里 DashScope",
        EmbeddingRoutesMessage::InsertAlibabaTemplate,
    ));

    assert_eq!(tree.children.len(), 1);
    assert!(tree.children[0].children.is_empty());
}

#[test]
fn embedded_view_empty_routes_shows_empty_panel_between_section_and_save_action() {
    let app = test_app();
    let tree = element_tree(embedded_view(&app.embedding_routes_settings, style()));
    let content = content_tree(&tree);

    assert_eq!(content.children.len(), 5);
    assert_eq!(content.children[0].children.len(), 2);
    assert_eq!(content.children[4].children.len(), 2);
}

#[test]
fn embedded_view_success_banner_requires_success_without_error() {
    let mut app = test_app();
    app.embedding_routes_settings.save_success = true;
    let success_tree = element_tree(embedded_view(&app.embedding_routes_settings, style()));
    assert_eq!(content_tree(&success_tree).children.len(), 6);

    app.embedding_routes_settings.save_error = Some(("保存失败".to_string()).into());
    let error_tree = element_tree(embedded_view(&app.embedding_routes_settings, style()));
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

    let tree = element_tree(embedded_view(&app.embedding_routes_settings, style()));
    let content = content_tree(&tree);

    assert_eq!(content.children.len(), 6);
    for card in &content.children[3..5] {
        assert_eq!(card.children.len(), 2);
        assert_eq!(card.children[0].children.len(), 3);
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

    let tree = element_tree(embedded_view(&app.embedding_routes_settings, style()));
    let card = &content_tree(&tree).children[3];
    let fields = &card.children[1];

    assert_eq!(fields.children.len(), 5);
    for field in &fields.children {
        let row = content_tree(field);
        assert_eq!(row.children.len(), 2);
        assert_eq!(row.children[0].children.len(), 2);
    }
}
