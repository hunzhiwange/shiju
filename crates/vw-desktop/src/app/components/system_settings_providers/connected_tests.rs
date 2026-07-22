// Tests for plan6 task 808 and task 632.
const SOURCE: &str = include_str!("connected.rs");

use crate::app::state::{ModelCatalogEntry, ProviderSummary};
use crate::app::{App, Message};
use iced::Theme;
use iced::advanced::widget::Tree;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn element_tree(element: iced::Element<'_, Message>) -> Tree {
    Tree::new(element.as_widget())
}

fn provider(id: &str, name: &str, connected: bool) -> ProviderSummary {
    ProviderSummary {
        id: id.to_string(),
        name: name.to_string(),
        source_label: "配置文件".to_string(),
        connected,
    }
}

fn catalog_entry(provider_id: &str, provider_name: &str) -> ModelCatalogEntry {
    ModelCatalogEntry {
        provider_id: provider_id.to_string(),
        provider_name: provider_name.to_string(),
        model_id: "model".to_string(),
        model_name: "Model".to_string(),
    }
}

fn source_declares_symbol(name: &str) -> bool {
    let needles = [
        format!("fn {name}"),
        format!("pub fn {name}"),
        format!("struct {name}"),
        format!("pub struct {name}"),
        format!("enum {name}"),
        format!("pub enum {name}"),
        format!("type {name}"),
        format!("pub type {name}"),
        format!("const {name}"),
        format!("pub const {name}"),
        format!("static {name}"),
        format!("pub static {name}"),
        format!("impl {name}"),
    ];

    needles.iter().any(|needle| SOURCE.contains(needle))
}

#[test]
fn view_builds_loading_sync_empty_connected_popular_and_error_states() {
    let mut app = test_app();
    keep_element(super::connected::view(&app));

    app.provider_settings.loading = true;
    keep_element(super::connected::view(&app));

    app.provider_settings.loading = false;
    app.provider_settings.models_syncing = true;
    app.provider_settings.models_sync_progress = 0.42;
    app.provider_settings.models_sync_label = "同步中".to_string();
    app.provider_settings.save_error = Some("保存失败".to_string());
    keep_element(super::connected::view(&app));

    app.provider_settings.models_syncing = false;
    app.provider_settings.providers =
        vec![provider("openai", "OpenAI", true), provider("anthropic", "Anthropic", false)];
    app.provider_settings.catalog_items =
        vec![catalog_entry("openai", "OpenAI"), catalog_entry("anthropic", "Anthropic")];
    app.provider_settings.popular_patterns =
        vec!["OpenAI".to_string(), "anth".to_string(), "missing".to_string()];
    keep_element(super::connected::view(&app));

    app.provider_settings.disconnect_confirm_provider_id = Some("openai".to_string());
    keep_element(super::connected::view(&app));

    app.provider_settings.popular_patterns.clear();
    keep_element(super::connected::view(&app));
}

#[test]
fn provider_action_panel_style_builds_with_dark_theme() {
    let style = super::connected::provider_action_panel_style(&Theme::Dark);
    std::hint::black_box(style);
}

#[test]
fn provider_item_row_builds_with_text_control() {
    let element = super::connected::provider_item_row(
        "test-logo",
        "Test Title",
        "Test description",
        iced::widget::text("control"),
    );
    keep_element(element);
}

#[test]
fn popular_provider_match_prefers_catalog_and_derives_connected_state_from_provider_list() {
    let catalogs = vec![catalog_entry("openai", "OpenAI"), catalog_entry("anthropic", "Anthropic")];
    let providers = vec![
        provider("openai", "OpenAI from config", true),
        provider("anthropic", "Anthropic from config", false),
    ];

    let openai = super::connected::popular_provider_match("open", &catalogs, &providers);
    assert_eq!(openai.display, "OpenAI");
    assert_eq!(openai.provider_id.as_deref(), Some("openai"));
    assert!(openai.is_connected);

    let anthropic = super::connected::popular_provider_match("ANTHROPIC", &catalogs, &providers);
    assert_eq!(anthropic.display, "Anthropic");
    assert_eq!(anthropic.provider_id.as_deref(), Some("anthropic"));
    assert!(!anthropic.is_connected);
}

#[test]
fn popular_provider_match_falls_back_to_existing_provider_or_missing_pattern() {
    let providers = vec![
        provider("custom-local", "Custom Local", false),
        provider("another-provider", "Another Provider", true),
    ];

    let custom = super::connected::popular_provider_match("local", &[], &providers);
    assert_eq!(custom.display, "Custom Local");
    assert_eq!(custom.provider_id.as_deref(), Some("custom-local"));
    assert!(!custom.is_connected);

    let missing = super::connected::popular_provider_match("missing", &[], &providers);
    assert_eq!(missing.display, "missing");
    assert_eq!(missing.provider_id, None);
    assert!(!missing.is_connected);
}

#[test]
fn view_builds_popular_pattern_matching_existing_provider_not_catalog() {
    let mut app = test_app();
    // providers without matching catalog entries
    app.provider_settings.providers = vec![
        provider("custom-local", "Custom Local", false),
        provider("another-provider", "Another Provider", true),
    ];
    app.provider_settings.catalog_items = vec![];
    // "custom" matches no catalog but matches "custom-local" by id_lower.contains
    // "another" matches no catalog but matches "another-provider" (connected)
    app.provider_settings.popular_patterns = vec!["custom".to_string(), "another".to_string()];
    keep_element(super::connected::view(&app));
}

#[test]
fn view_inserts_dividers_between_multiple_connected_and_popular_rows() {
    let mut app = test_app();
    app.provider_settings.providers =
        vec![provider("openai", "OpenAI", true), provider("anthropic", "Anthropic", true)];
    app.provider_settings.catalog_items =
        vec![catalog_entry("openai", "OpenAI"), catalog_entry("anthropic", "Anthropic")];
    app.provider_settings.popular_patterns = vec!["openai".to_string(), "anthropic".to_string()];

    let tree = element_tree(super::connected::view(&app));

    assert_eq!(tree.children.len(), 8);
    assert_eq!(tree.children[2].children.len(), 3);
    assert_eq!(tree.children[4].children.len(), 3);
}

#[test]
fn connected_tests_keeps_planned_coverage_targets() {
    for name in ["provider_action_panel_style", "provider_item_row", "view"] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}
