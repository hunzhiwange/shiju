use crate::model_selector::{SelectorMessage, SelectorView};
use desktop_settings_providers::state::{ModelSettingsState, ModelSummary, ProviderModelsSummary};

#[derive(Default)]
struct ModelFixture {
    desktop_locale: DesktopLocale,
    task_board_bulk_model_popover: bool,
    task_board_bulk_model_input: String,
    model_settings: ModelSettingsState,
}
impl ModelFixture {
    fn view(&self) -> SelectorView<'_> {
        SelectorView {
            draft_model: "auto",
            bulk_model: &self.task_board_bulk_model_input,
            task_board_model_popover: false,
            task_board_bulk_model_popover: self.task_board_bulk_model_popover,
            desktop_locale: self.desktop_locale,
            model_settings: &self.model_settings,
        }
    }
}
use super::ui::*;
use config_ui::DesktopLocale;
use desktop_i18n::I18n;
use iced::keyboard::{Key, key::Named};
use iced::{Point, Size, Theme, mouse};
use iced_runtime::user_interface::UserInterface;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_35_model_search_groups_and_bulk_selection() {
    let mut app = ModelFixture::default();
    let mut renderer = renderer().await;
    app.desktop_locale = DesktopLocale::English;
    app.task_board_bulk_model_popover = true;
    app.task_board_bulk_model_input = "fixture/saved-model".into();
    app.model_settings.loading = true;
    let size = Size::new(420, 480);
    let theme = Theme::Light;
    let i18n = I18n::new(app.desktop_locale);
    for (loading, name, label) in [
        (true, "model-loading", "加载中…"),
        (false, "model-empty", "暂无可选模型（请先在系统设置里启用模型）"),
    ] {
        app.model_settings.loading = loading;
        let mut ui = scene(
            crate::model_selector::build_bulk_model_selector(app.view()),
            &mut renderer,
            size,
        );
        capture(&mut ui, &mut renderer, &theme, size, name, mouse::Cursor::Unavailable);
        assert!(labels(&mut ui, &renderer).bounds(i18n.__(label)).height > 0.0);
    }

    app.model_settings.providers = vec![
        ProviderModelsSummary {
            id: "fixture".into(),
            name: "Fixture models".into(),
            models: vec![ModelSummary {
                id: "saved-model".into(),
                name: format!("A {}", "很长的模型標籤".repeat(16)),
                enabled: true,
                toolcall: true,
                attachment: false,
                context_limit: 32000,
                detail: serde_json::json!({}),
            }],
        },
        ProviderModelsSummary {
            id: "other".into(),
            name: "Other models".into(),
            models: vec![ModelSummary {
                id: "other-model".into(),
                name: "Other model".into(),
                enabled: true,
                toolcall: false,
                attachment: false,
                context_limit: 32000,
                detail: serde_json::json!({}),
            }],
        },
    ];
    let mut ui =
        scene(crate::model_selector::build_bulk_model_selector(app.view()), &mut renderer, size);
    capture(&mut ui, &mut renderer, &theme, size, "model-groups", mouse::Cursor::Unavailable);
    let geometry = labels(&mut ui, &renderer);
    assert!(geometry.bounds("Other models").height > 0.0);
    let search = geometry.input(i18n.__("搜索模型…"));
    assert!(click(&mut ui, &mut renderer, search.center()).is_empty());
    assert!(matches!(
        key(&mut ui, &mut renderer, Key::Character("f".into()), Some("f")).as_slice(),
        [SelectorMessage::ModelQueryChanged(value)] if value == "f"
    ));
    let cache = ui.into_cache();
    app.model_settings.query = "fixture".into();
    let mut ui = UserInterface::build(
        crate::model_selector::build_bulk_model_selector(app.view()),
        Size::new(420.0, 480.0),
        cache,
        &mut renderer,
    );
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::End), None).is_empty());
    assert!(matches!(
        key(&mut ui, &mut renderer, Key::Character("x".into()), Some("x")).as_slice(),
        [SelectorMessage::ModelQueryChanged(value)] if value == "fixturex"
    ));
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "model-filtered",
        mouse::Cursor::Unavailable,
    );
    let geometry = labels(&mut ui, &renderer);
    let manual = geometry.input("fixture/saved-model");
    assert!(click(&mut ui, &mut renderer, manual.center()).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::End), None).is_empty());
    assert!(matches!(
        key(&mut ui, &mut renderer, Key::Character("x".into()), Some("x")).as_slice(),
        [SelectorMessage::UpdateBulkModelInput(value)]
            if value == "fixture/saved-modelx"
    ));
    assert!(!geometry.text.iter().any(|(value, _)| value == "Other models"));
    let (menu, _) = geometry.scroll("task_board_model_selector_scroll");
    assert!(menu.y + menu.height <= 480.0);
    let group = geometry.bounds("Fixture models");
    let row_y = group.y + group.height + 4.0 + 2.0 + 18.0;
    assert!(matches!(
        click(&mut ui, &mut renderer, Point::new(menu.x + menu.width - 24.0, row_y)).as_slice(),
        [SelectorMessage::OpenModelDetail(provider, model)]
            if provider == "fixture" && model == "saved-model"
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, Point::new(menu.center_x(), row_y)).as_slice(),
        [SelectorMessage::BulkModelSelected(value)] if value == "fixture/saved-model"
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, Point::new(416.0, 476.0)).as_slice(),
        [SelectorMessage::CloseBulkModelPopover]
    ));
}
