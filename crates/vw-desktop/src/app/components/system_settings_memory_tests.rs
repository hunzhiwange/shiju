use super::*;
use crate::app::App;
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
fn system_settings_memory_tests_are_wired() {
    assert!(module_path!().contains("system_settings_memory_tests"));
}

#[test]
fn parse_bounded_number_uses_value_fallback_and_limits() {
    assert_eq!(parse_bounded_number("12.5", 1.0, 20.0), 12.5);
    assert_eq!(parse_bounded_number("invalid", 1.0, 20.0), 1.0);
    assert_eq!(parse_bounded_number("-4", 1.0, 20.0), 1.0);
    assert_eq!(parse_bounded_number("25", 1.0, 20.0), 20.0);
}

#[test]
fn parse_bounded_number_clamps_infinite_values_and_preserves_bounds() {
    assert_eq!(parse_bounded_number("inf", 1.0, 20.0), 20.0);
    assert_eq!(parse_bounded_number("-inf", 1.0, 20.0), 1.0);
    assert_eq!(parse_bounded_number("1", 1.0, 20.0), 1.0);
    assert_eq!(parse_bounded_number("20", 1.0, 20.0), 20.0);
}

#[test]
fn row_helpers_build_expected_control_structure() {
    let field = element_tree(field_row("标签", "说明", text("control")));
    assert_eq!(field.children.len(), 2);
    assert_eq!(field.children[0].children.len(), 2);
    assert!(field.children[1].children.is_empty());

    let text = element_tree(text_row("地址", "说明", "placeholder", "value", |_| {
        Message::Settings(SettingsMessage::Memory(MemoryMessage::QdrantUrlChanged(
            "changed".to_string(),
        )))
    }));
    assert!(text.children[1].children.is_empty());

    let boolean = element_tree(bool_row("启用", "说明", true, "启用记忆", |value| {
        Message::Settings(SettingsMessage::Memory(MemoryMessage::AutoSaveToggled(value)))
    }));
    assert!(boolean.children[1].children.is_empty());

    let number = element_tree(number_row(
        "数量",
        "invalid".to_string(),
        "条",
        0.0,
        100.0,
        1.0,
        0,
        |value| {
            Message::Settings(SettingsMessage::Memory(MemoryMessage::EmbeddingCacheSizeChanged(
                value.round() as u32,
            )))
        },
    ));
    assert_eq!(number.children[1].children.len(), 2);
}

#[test]
fn view_shows_qdrant_section_only_for_qdrant_backend() {
    let mut app = test_app();

    for backend in ["sqlite", "postgres", "chroma", "markdown", "null", "unknown"] {
        app.memory_settings.backend = backend.to_string();
        assert_eq!(element_tree(view(&app)).children.len(), 4, "backend={backend}");
    }

    app.memory_settings.backend = "qdrant".to_string();
    assert_eq!(element_tree(view(&app)).children.len(), 5);
}

#[test]
fn view_builds_intro_and_two_part_sections_for_default_and_qdrant_backends() {
    let mut app = test_app();
    app.memory_settings.backend = "sqlite".to_string();

    let sqlite = element_tree(view(&app));
    assert_eq!(sqlite.children.len(), 4);
    assert_eq!(sqlite.children[0].children.len(), 2);
    for section in sqlite.children.iter().skip(1) {
        assert_eq!(section.children.len(), 2);
    }

    app.memory_settings.backend = "qdrant".to_string();
    let qdrant = element_tree(view(&app));
    assert_eq!(qdrant.children.len(), 5);
    assert_eq!(qdrant.children[4].children.len(), 2);
}

#[test]
fn view_builds_boundary_memory_state() {
    let mut custom = test_app();

    custom.memory_settings.backend = "qdrant".to_string();
    custom.memory_settings.auto_save = false;
    custom.memory_settings.hygiene_enabled = false;
    custom.memory_settings.response_cache_enabled = true;
    custom.memory_settings.snapshot_enabled = true;
    custom.memory_settings.snapshot_on_hygiene = true;
    custom.memory_settings.auto_hydrate = false;
    custom.memory_settings.archive_after_days = 0;
    custom.memory_settings.purge_after_days = 3650;
    custom.memory_settings.conversation_retention_days = 90;
    custom.memory_settings.embedding_provider = "hint:semantic".to_string();
    custom.memory_settings.embedding_model = "text-embedding-v4".to_string();
    custom.memory_settings.embedding_dimensions = 2048;
    custom.memory_settings.vector_weight = 1.0;
    custom.memory_settings.keyword_weight = 0.0;
    custom.memory_settings.min_relevance_score = 0.95;
    custom.memory_settings.embedding_cache_size = 0;
    custom.memory_settings.chunk_max_tokens = 32_768;
    custom.memory_settings.response_cache_ttl_minutes = 0;
    custom.memory_settings.response_cache_max_entries = 0;
    custom.memory_settings.sqlite_open_timeout_secs = 3600;
    custom.memory_settings.qdrant_url_input = "http://localhost:6333".to_string();
    custom.memory_settings.qdrant_collection = "memories".to_string();
    custom.memory_settings.qdrant_api_key_input = "secret".to_string();
    custom.memory_settings.save_error = Some("memory save failed".to_string());

    assert_eq!(element_tree(view(&custom)).children.len(), 5);
}
