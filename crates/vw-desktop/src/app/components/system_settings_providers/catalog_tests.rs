use crate::app::state::{ModelCatalogEntry, ProviderSummary};
use crate::app::{App, Message};
use iced::widget::text;
use iced::{Background, Color, Theme, Vector};

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn catalog_entry(provider_id: &str, provider_name: &str, model_id: &str) -> ModelCatalogEntry {
    ModelCatalogEntry {
        provider_id: provider_id.to_string(),
        provider_name: provider_name.to_string(),
        model_id: model_id.to_string(),
        model_name: model_id.to_string(),
    }
}

fn provider(id: &str, connected: bool) -> ProviderSummary {
    ProviderSummary {
        id: id.to_string(),
        name: id.to_string(),
        source_label: "测试配置".to_string(),
        connected,
    }
}

#[test]
fn overlays_return_base_when_catalog_is_closed() {
    let app = test_app();

    keep_element(super::catalog::view_overlays(&app, text("dialog").into()));
}

#[test]
fn overlays_build_loading_and_empty_catalogs() {
    let mut app = test_app();
    app.provider_settings.catalog_open = true;
    app.provider_settings.catalog_loading = true;
    keep_element(super::catalog::view_overlays(&app, text("dialog").into()));

    app.provider_settings.catalog_loading = false;
    keep_element(super::catalog::view_overlays(&app, text("dialog").into()));
}

#[test]
fn overlays_build_filtered_and_populated_catalogs() {
    let mut app = test_app();
    app.provider_settings.catalog_open = true;
    app.provider_settings.catalog_items = vec![
        catalog_entry("openai", "OpenAI", "gpt-5"),
        catalog_entry("openai", "OpenAI", "gpt-5-mini"),
        catalog_entry("anthropic", "Anthropic", "claude-4"),
    ];
    app.provider_settings.providers = vec![ProviderSummary {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        source_label: "配置文件".to_string(),
        connected: true,
    }];
    app.provider_settings.popular_patterns = vec!["anthropic".to_string()];
    keep_element(super::catalog::view_overlays(&app, text("dialog").into()));

    app.provider_settings.catalog_query = "anth".to_string();
    keep_element(super::catalog::view_overlays(&app, text("dialog").into()));

    app.provider_settings.catalog_query = "missing".to_string();
    keep_element(super::catalog::view_overlays(&app, text("dialog").into()));
}

#[test]
fn visible_catalog_providers_groups_models_and_sorts_by_provider_id() {
    let items = vec![
        catalog_entry("zeta", "Zeta", "z-1"),
        catalog_entry("alpha", "Alpha", "a-1"),
        catalog_entry("zeta", "Zeta", "z-2"),
    ];

    let visible = super::catalog::visible_catalog_providers(&items, &[], &[], "");

    assert_eq!(
        visible
            .iter()
            .map(|provider| (provider.provider_id.as_str(), provider.model_count))
            .collect::<Vec<_>>(),
        vec![("alpha", 1), ("zeta", 2)]
    );
}

#[test]
fn visible_catalog_providers_keeps_first_name_when_grouping_duplicate_provider_ids() {
    let items = vec![
        catalog_entry("shared", "First Name", "model-a"),
        catalog_entry("shared", "Later Name", "model-b"),
    ];

    let visible = super::catalog::visible_catalog_providers(&items, &[], &[], "");

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].provider_name, "First Name");
    assert_eq!(visible[0].model_count, 2);
}

#[test]
fn visible_catalog_providers_matches_trimmed_query_by_id_and_name_case_insensitively() {
    let items = vec![
        catalog_entry("openai-compatible", "Custom Gateway", "model-a"),
        catalog_entry("anthropic", "Anthropic", "model-b"),
    ];

    let by_id = super::catalog::visible_catalog_providers(&items, &[], &[], "  COMPATIBLE ");
    let by_name = super::catalog::visible_catalog_providers(&items, &[], &[], "gateway");
    let missing = super::catalog::visible_catalog_providers(&items, &[], &[], "missing");

    assert_eq!(by_id[0].provider_id, "openai-compatible");
    assert_eq!(by_name[0].provider_name, "Custom Gateway");
    assert!(missing.is_empty());
}

#[test]
fn visible_catalog_providers_marks_connected_and_popular_independently() {
    let items = vec![
        catalog_entry("connected", "Connected", "model-a"),
        catalog_entry("popular", "Popular", "model-b"),
        catalog_entry("disconnected", "Disconnected", "model-c"),
    ];
    let configured = vec![provider("connected", true), provider("disconnected", false)];
    let popular = vec!["popular".to_string()];

    let visible = super::catalog::visible_catalog_providers(&items, &configured, &popular, "");

    let connected = visible.iter().find(|provider| provider.provider_id == "connected").unwrap();
    let popular = visible.iter().find(|provider| provider.provider_id == "popular").unwrap();
    let disconnected =
        visible.iter().find(|provider| provider.provider_id == "disconnected").unwrap();
    assert!(connected.is_connected);
    assert!(!connected.in_popular);
    assert!(!popular.is_connected);
    assert!(popular.in_popular);
    assert!(!disconnected.is_connected);
    assert!(!disconnected.in_popular);
}

#[test]
fn model_count_label_formats_singular_and_plural_counts() {
    assert_eq!(super::catalog::model_count_label(1), "1 个模型");
    assert_eq!(super::catalog::model_count_label(0), "0 个模型");
    assert_eq!(super::catalog::model_count_label(12), "12 个模型");
}

#[test]
fn catalog_surface_style_uses_dark_theme_palette_and_expected_border() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();
    let style = super::catalog::catalog_surface_style(&theme);

    assert_eq!(
        style.background,
        Some(Background::Color(palette.background.weak.color.scale_alpha(0.16)))
    );
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.color, palette.background.strong.color.scale_alpha(0.76));
    assert_eq!(style.border.radius, 18.0.into());
}

#[test]
fn catalog_list_frame_style_uses_dark_theme_palette_and_shadow() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();
    let style = super::catalog::catalog_list_frame_style(&theme);

    assert_eq!(
        style.background,
        Some(Background::Color(palette.background.base.color.scale_alpha(0.84)))
    );
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.color, palette.background.strong.color.scale_alpha(0.92));
    assert_eq!(style.border.radius, 20.0.into());
    assert_eq!(style.shadow.color, Color::BLACK.scale_alpha(0.04));
    assert_eq!(style.shadow.offset, Vector::new(0.0, 2.0));
    assert_eq!(style.shadow.blur_radius, 8.0);
    assert!(!style.snap);
}

#[test]
fn catalog_item_style_uses_dark_theme_palette_and_shadow() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();
    let style = super::catalog::catalog_item_style(&theme);

    assert_eq!(
        style.background,
        Some(Background::Color(palette.background.base.color.scale_alpha(0.72)))
    );
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.color, palette.background.strong.color.scale_alpha(0.82));
    assert_eq!(style.border.radius, 14.0.into());
    assert_eq!(style.shadow.color, Color::BLACK.scale_alpha(0.05));
    assert_eq!(style.shadow.offset, Vector::new(0.0, 3.0));
    assert_eq!(style.shadow.blur_radius, 10.0);
    assert!(!style.snap);
}
