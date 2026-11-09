use super::{catalog, connect, connected, custom_provider};
use crate::app::message::SettingsMessage;
use crate::app::state::{ModelCatalogEntry, ProviderConnectState, ProviderSummary};
use crate::app::{App, Message};
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::widget::operation::{Operation, Scrollable};
use iced::advanced::{Layout, clipboard, layout, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::DesktopLocale;

fn fixture() -> App {
    let mut app = None;
    crate::app::config::with_editor_settings_gateway(|fixture, _, _| app = Some(fixture));
    let mut app = app.expect("isolated settings fixture");
    app.desktop_locale = DesktopLocale::English;
    let s = &mut app.provider_settings;
    s.catalog_items = (0..14)
        .map(|i| ModelCatalogEntry {
            provider_id: format!("fixture-{i:02}-with-a-long-provider-identifier"),
            provider_name: format!("Fixture {i:02} 虚构模型供應商 — A long provider display name"),
            model_id: "fixture-model".into(),
            model_name: "Fixture model".into(),
        })
        .collect();
    s.providers = vec![ProviderSummary {
        id: s.catalog_items[0].provider_id.clone(),
        name: s.catalog_items[0].provider_name.clone(),
        source_label: "Fixture source".into(),
        connected: true,
    }];
    s.popular_patterns = vec![s.providers[0].id.clone(), "missing-fixture".into()];
    app
}

async fn renderer() -> Renderer {
    static FONTS: std::sync::Once = std::sync::Once::new();
    FONTS.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    });
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .unwrap()
}

fn capture(
    element: Element<'_, Message>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
) -> Vec<u8> {
    let mut ui = UserInterface::build(
        element,
        Size::new(size.width as f32, size.height as f32),
        Cache::new(),
        renderer,
    );
    let cursor = mouse::Cursor::Unavailable;
    ui.update(
        &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
        cursor,
        renderer,
        &mut iced::advanced::clipboard::Null,
        &mut Vec::new(),
    );
    ui.draw(renderer, theme, &Style { text_color: theme.palette().text }, cursor);
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    if let Some(directory) = std::env::var_os("SHIJU_SETTINGS_PREVIEW_DIR") {
        image::save_buffer(
            std::path::PathBuf::from(directory).join(format!("{name}.png")),
            &pixels,
            size.width,
            size.height,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
    pixels
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn help_rollout_05_renders_contextual_help() {
    use crate::app::state::CustomProviderTab;

    let mut renderer = renderer().await;
    let mut app = fixture();
    for (locale, theme, mode) in [
        (DesktopLocale::English, Theme::Light, "en-light"),
        (DesktopLocale::SimplifiedChinese, Theme::Dark, "zh-dark"),
        (DesktopLocale::TraditionalChinese, Theme::Light, "zh-hant-light"),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        assert_help(connected::view(&app), &mut renderer, &theme, Size::new(560, 940), 5, mode);

        app.provider_settings.catalog_open = true;
        assert_help(
            catalog::view_overlays(&app, background()),
            &mut renderer,
            &theme,
            Size::new(420, 620),
            1,
            &format!("catalog-{mode}"),
        );
        app.provider_settings.catalog_open = false;

        app.provider_settings.connect_modal = Some(ProviderConnectState {
            provider_id: "fixture".into(),
            provider_name: "Fixture 提供商".into(),
            api_key: "secret-fixture".into(),
        });
        app.provider_settings.connect_error = Some("Fixture credential error".into());
        assert_help(
            connect::view_overlays(&app, background()),
            &mut renderer,
            &theme,
            Size::new(320, 440),
            1,
            &format!("connect-{mode}"),
        );
        app.provider_settings.connect_modal = None;

        app.provider_settings.custom_provider_modal_open = true;
        app.provider_settings.custom.provider_id = "fixture".into();
        app.provider_settings.custom.base_url = "https://private-fixture.invalid/v1".into();
        app.provider_settings.custom.api_key = "secret-fixture".into();
        for (tab, count) in [
            (CustomProviderTab::Basic, 5),
            (CustomProviderTab::Authentication, 4),
            (CustomProviderTab::Models, 4),
        ] {
            app.provider_settings.custom_provider_tab = tab;
            assert_help(
                custom_provider::view_overlays(&app, background()),
                &mut renderer,
                &theme,
                Size::new(420, 620),
                count,
                &format!("custom-{tab:?}-{mode}"),
            );
        }
        app.provider_settings.custom_editing_provider_id = Some("fixture".into());
        app.provider_settings.custom_provider_tab = CustomProviderTab::Basic;
        assert_help(
            custom_provider::view_overlays(&app, background()),
            &mut renderer,
            &theme,
            Size::new(420, 620),
            5,
            &format!("edit-{mode}"),
        );
        app.provider_settings.custom_editing_provider_id = None;
        app.provider_settings.custom_provider_modal_open = false;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn help_rollout_05_keeps_credentials_and_validation_outside_help() {
    use crate::app::i18n::settings_models::connection_text::text as tr;
    use crate::app::message::ViewMessage;

    let mut renderer = renderer().await;
    let mut app = fixture();
    app.app_theme = Theme::Light;
    app.provider_settings.connect_modal = Some(ProviderConnectState {
        provider_id: "fixture-private-id".into(),
        provider_name: "Fixture Private Provider".into(),
        api_key: "fixture-private-key".into(),
    });

    let mut scene = Scene::new(
        connect::view_overlays(&app, background()),
        &mut renderer,
        Size::new(700.0, 620.0),
    );
    let icon = scene.bounds(&modal_path(&[1, 0, 0, 0, 0, 1]));
    assert!(scene.click(&mut renderer, icon.center()).is_empty());
    let card_text = Point::new(icon.x + icon.width + 4.0 + 18.0, icon.y + 16.0);
    let menu_row_height =
        f32::from(iced::advanced::text::LineHeight::default().to_absolute(12.0.into())) + 10.0;
    let menu_item = |row: f32| {
        Point::new(card_text.x + 24.0, card_text.y + 3.0 + menu_row_height * (row + 0.5))
    };
    let description = tr(app.desktop_locale, "录入并保存当前 Provider 的 API 密钥。");
    let expected_url =
        format!("https://www.google.com/search?q={}", urlencoding::encode(description));
    for (row, copy) in [(0.0, true), (3.0, false)] {
        assert!(
            scene
                .send(
                    &mut renderer,
                    &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))],
                    card_text,
                )
                .is_empty(),
            "opening the menu has no external action"
        );
        let messages = scene.click(&mut renderer, menu_item(row));
        assert!(match messages.as_slice() {
            [Message::CopyCode(text)] if copy => text == description,
            [Message::View(ViewMessage::OpenUrlExternal(url))] if !copy => url == &expected_url,
            _ => false,
        });
    }
    assert!(scene.send(&mut renderer, &[key(Named::Escape)], icon.center()).is_empty());
    drop(scene);

    app.provider_settings.connect_modal.as_mut().unwrap().api_key.clear();
    let mut scene = Scene::new(
        connect::view_overlays(&app, background()),
        &mut renderer,
        Size::new(420.0, 440.0),
    );
    let save = scene.bounds(&modal_path(&[2, 0, 1])).center();
    let messages = scene.click(&mut renderer, save);
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::ProviderConnectSubmit)]
    ));
    drop(scene);
    for message in messages {
        let _ = app.update(message);
    }
    assert_eq!(
        app.provider_settings.connect_error.as_ref().unwrap().display(app.desktop_locale),
        tr(app.desktop_locale, "请输入 API 密钥")
    );
    assert!(app.provider_settings.connect_modal.is_some());

    app.provider_settings.connect_modal = None;
    app.provider_settings.custom_provider_modal_open = true;
    for (id, editing, expected_error) in [
        ("INVALID", None, "提供商 ID 仅支持小写字母、数字、连字符或下划线"),
        ("fixture", Some("fixture".into()), "基础 URL 不能为空"),
    ] {
        app.provider_settings.custom.provider_id = id.into();
        app.provider_settings.custom.base_url.clear();
        app.provider_settings.custom_editing_provider_id = editing;
        let mut scene = Scene::new(
            custom_provider::view_overlays(&app, background()),
            &mut renderer,
            Size::new(420.0, 620.0),
        );
        let save = scene.bounds(&modal_path(&[2, 2])).center();
        let messages = scene.click(&mut renderer, save);
        assert!(matches!(
            messages.as_slice(),
            [Message::Settings(SettingsMessage::CustomProviderSave)]
        ));
        drop(scene);
        for message in messages {
            let _ = app.update(message);
        }
        assert_eq!(
            app.provider_settings.save_error.as_ref().unwrap().display(app.desktop_locale),
            tr(app.desktop_locale, expected_error)
        );
        assert!(app.provider_settings.custom_provider_modal_open);
    }
    app.provider_settings.custom_models_discovering = true;
    let mut scene = Scene::new(
        custom_provider::view_overlays(&app, background()),
        &mut renderer,
        Size::new(420.0, 620.0),
    );
    let save = scene.bounds(&modal_path(&[2, 2])).center();
    let cancel = scene.bounds(&modal_path(&[2, 1])).center();
    assert!(scene.click(&mut renderer, save).is_empty());
    assert!(matches!(
        scene.click(&mut renderer, cancel).as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderClose)]
    ));
}

fn assert_help(
    element: Element<'_, Message>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    count: usize,
    name: &str,
) {
    fn collect_icons(layout: Layout<'_>, icons: &mut Vec<Rectangle>) {
        let bounds = layout.bounds();
        if bounds.size() == Size::new(22.0, 22.0) && !icons.contains(&bounds) {
            icons.push(bounds);
        }
        for child in layout.children() {
            collect_icons(child, icons);
        }
    }

    let mut scene = Scene::new(element, renderer, Size::new(size.width as f32, size.height as f32));
    let mut icons = Vec::new();
    collect_icons(Layout::new(&scene.layout), &mut icons);
    assert_eq!(icons.len(), count, "{name} contextual help targets");
    let frame = |scene: &mut Scene<'_>, renderer: &mut Renderer, cursor, suffix: &str| {
        scene.draw(renderer, theme, cursor);
        let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
        if let Some(directory) = std::env::var_os("SHIJU_SETTINGS_PREVIEW_DIR") {
            image::save_buffer(
                std::path::PathBuf::from(directory).join(format!("help-05-{name}-{suffix}.png")),
                &pixels,
                size.width,
                size.height,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
        pixels
    };
    let idle = frame(&mut scene, renderer, mouse::Cursor::Unavailable, "idle");
    for (index, icon) in icons.into_iter().enumerate() {
        let point = icon.center();
        assert!(point.x >= 0.0 && point.x <= size.width as f32, "{name} icon in window");
        assert!(
            scene
                .send(
                    renderer,
                    &[Event::Mouse(mouse::Event::CursorMoved { position: point })],
                    point
                )
                .is_empty()
        );
        let open =
            frame(&mut scene, renderer, mouse::Cursor::Available(point), &format!("open-{index}"));
        assert!(idle != open, "{name} icon {index} opens a rendered help card");
        assert!(scene.send(renderer, &[key(Named::Escape)], point).is_empty());
        assert!(
            idle == frame(&mut scene, renderer, mouse::Cursor::Unavailable, "dismissed"),
            "{name} icon {index} dismisses without changing the parent",
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_08_layout_and_interaction() {
    let mut renderer = renderer().await;
    let mut app = fixture();
    for (locale, suffix) in [
        (DesktopLocale::SimplifiedChinese, "zh"),
        (DesktopLocale::TraditionalChinese, "zh-hant"),
        (DesktopLocale::English, "en"),
    ] {
        app.desktop_locale = locale;
        for (theme, mode, width) in [(Theme::Light, "light", 640), (Theme::Dark, "dark", 960)] {
            capture(
                connected::view(&app),
                &mut renderer,
                &theme,
                Size::new(width, 980),
                &format!("providers-{suffix}-{mode}-{width}"),
            );
            app.provider_settings.catalog_open = true;
            capture(
                catalog::view_overlays(&app, connected::view(&app)),
                &mut renderer,
                &theme,
                Size::new(if theme == Theme::Light { 420 } else { width }, 620),
                &format!("catalog-{suffix}-{mode}"),
            );
            app.provider_settings.catalog_open = false;
        }
    }

    let page = Size::new(640.0, 980.0);
    let expected = app.provider_settings.providers[0].id.clone();
    let mut scene = Scene::new(connected::view(&app), &mut renderer, page);
    for (path, expected) in [
        (&[0, 1][..], SettingsMessage::ProviderModelsSyncRemote),
        (&[0, 2][..], SettingsMessage::ProvidersRefresh),
        (&[5, 0, 1][..], SettingsMessage::ProviderCatalogOpen),
    ] {
        let actual = scene.click(&mut renderer, scene.bounds(path).center());
        assert!(matches!(
            (actual.as_slice(), expected),
            (
                [Message::Settings(SettingsMessage::ProviderModelsSyncRemote)],
                SettingsMessage::ProviderModelsSyncRemote
            ) | (
                [Message::Settings(SettingsMessage::ProvidersRefresh)],
                SettingsMessage::ProvidersRefresh
            ) | (
                [Message::Settings(SettingsMessage::ProviderCatalogOpen)],
                SettingsMessage::ProviderCatalogOpen
            )
        ));
    }
    let controls: &[usize] = &[2, 0, 0, 0, 1, 0];
    let disconnect = scene.bounds(&[controls, &[3]].concat());
    assert!(disconnect.width > 80.0 && disconnect.x + disconnect.width <= page.width);
    assert!(matches!(
        scene.click(&mut renderer, disconnect.center()).as_slice(),
        [Message::Settings(SettingsMessage::ProviderDisconnectRequested(id))] if id == &expected
    ));
    drop(scene);

    app.provider_settings.disconnect_confirm_provider_id = Some(expected.clone());
    let mut scene = Scene::new(connected::view(&app), &mut renderer, page);
    assert!(matches!(
        scene.click(&mut renderer, scene.bounds(&[controls, &[1]].concat()).center()).as_slice(),
        [Message::Settings(SettingsMessage::ProviderDisconnectCanceled)]
    ));
    assert!(matches!(
        scene.click(&mut renderer, scene.bounds(&[controls, &[2]].concat()).center()).as_slice(),
        [Message::Settings(SettingsMessage::ProviderDisconnectConfirmed(id))] if id == &expected
    ));
    drop(scene);

    app.provider_settings.catalog_open = true;
    app.provider_settings.catalog_query = "FIXTURE-01".into();
    let expected = app.provider_settings.catalog_items[1].provider_id.clone();
    let mut scene = Scene::new(
        catalog::view_overlays(&app, background()),
        &mut renderer,
        Size::new(420.0, 620.0),
    );
    let search = scene.bounds(&modal_path(&[1]));
    assert!(search.width >= 340.0);
    let search_text = scene.bounds(&modal_path(&[1, 0]));
    assert!((search.center().y - search_text.center().y).abs() < 0.5);
    assert!(scene.click(&mut renderer, search.center()).is_empty());
    assert!(matches!(
        scene.send(&mut renderer, &[ime("-測試")], search.center()).as_slice(),
        [Message::Settings(SettingsMessage::ProviderCatalogQueryChanged(value))]
            if value == "FIXTURE-01-測試"
    ));
    assert!(
        scene
            .send(&mut renderer, &[key(Named::Enter), key(Named::Escape)], search.center())
            .is_empty()
    );

    let actions: &[usize] = &[2, 0, 0, 0, 0, 0, 0, 1, 0];
    let connect = scene.bounds(&modal_path(&[actions, &[0]].concat()));
    let favorite = scene.bounds(&modal_path(&[actions, &[1]].concat()));
    let logo = scene.bounds(&modal_path(&[2, 0, 0, 0, 0, 0, 0, 0, 0]));
    let logo_glyph = scene.bounds(&modal_path(&[2, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
    assert!((logo.center().y - logo_glyph.center().y).abs() < 0.5);
    assert!(connect.x > 12.0 && favorite.x + favorite.width < 408.0);
    assert!((connect.center().y - favorite.center().y).abs() < 0.5);
    for cursor in [connect.center(), favorite.center(), search.center()] {
        assert!(
            scene
                .send(
                    &mut renderer,
                    &[Event::Mouse(mouse::Event::CursorMoved { position: cursor })],
                    cursor,
                )
                .is_empty()
        );
        scene.draw(&mut renderer, &Theme::Dark, mouse::Cursor::Available(cursor));
    }
    assert!(matches!(
        scene.click(&mut renderer, connect.center()).as_slice(),
        [Message::Settings(SettingsMessage::ProviderConnectOpen(id))] if id == &expected
    ));
    assert!(matches!(
        scene.click(&mut renderer, favorite.center()).as_slice(),
        [Message::Settings(SettingsMessage::ProviderCatalogAddToPopular(id))] if id == &expected
    ));
    let backdrop = Point::new(2.0, 2.0);
    scene.draw(&mut renderer, &Theme::Dark, mouse::Cursor::Unavailable);
    let before = renderer.screenshot(Size::new(420, 620), 1.0, Theme::Dark.palette().background);
    assert!(
        scene
            .send(
                &mut renderer,
                &[Event::Mouse(mouse::Event::CursorMoved { position: backdrop })],
                backdrop,
            )
            .is_empty()
    );
    scene.draw(&mut renderer, &Theme::Dark, mouse::Cursor::Available(backdrop));
    let after = renderer.screenshot(Size::new(420, 620), 1.0, Theme::Dark.palette().background);
    assert!(before == after, "the backdrop must suppress the underlying button's hover");
    assert!(scene.click(&mut renderer, backdrop).is_empty());
    drop(scene);

    app.provider_settings.catalog_query.clear();
    let mut scene = Scene::new(
        catalog::view_overlays(&app, background()),
        &mut renderer,
        Size::new(420.0, 620.0),
    );
    let body = scene.bounds(&modal_path(&[2]));
    assert!(
        scene
            .send(
                &mut renderer,
                &[Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -170.0 }
                })],
                body.center(),
            )
            .is_empty()
    );
    let mut offset = ScrollOffset::default();
    scene.ui.operate(&renderer, &mut offset);
    assert!(offset.0.abs() > 100.0, "wheel must scroll the catalog body");
    let close = scene.bounds(&modal_path(&[0, 1]));
    assert!(matches!(
        scene.click(&mut renderer, close.center()).as_slice(),
        [Message::Settings(SettingsMessage::ProviderCatalogClose)]
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_08_preserves_values_and_disabled_states() {
    let mut renderer = renderer().await;
    let mut app = fixture();
    let p = &app.provider_settings.providers[0];
    app.provider_settings.connect_modal = Some(ProviderConnectState {
        provider_id: p.id.clone(),
        provider_name: p.name.clone(),
        api_key: "IIIIIIIIII".into(),
    });
    let first = capture(
        connect::view_overlays(&app, connected::view(&app)),
        &mut renderer,
        &Theme::Dark,
        Size::new(420, 440),
        "credential-first",
    );
    app.provider_settings.connect_modal.as_mut().unwrap().api_key = "WWWWWWWWWW".into();
    let second = capture(
        connect::view_overlays(&app, connected::view(&app)),
        &mut renderer,
        &Theme::Dark,
        Size::new(420, 440),
        "credential-second",
    );
    assert!(first == second, "keys of equal length must render the same masked glyphs");

    app.provider_settings.connect_modal.as_mut().unwrap().api_key.clear();
    app.provider_settings.connect_error = Some("连接失败：虛構憑證已失效".into());
    let mut scene = Scene::new(
        connect::view_overlays(&app, background()),
        &mut renderer,
        Size::new(420.0, 440.0),
    );
    let field = scene.bounds(&modal_path(&[1, 0, 0, 0, 1]));
    let error = scene.bounds(&modal_path(&[1, 0, 0, 0, 2]));
    assert!(error.y - (field.y + field.height) <= 8.5);
    let cancel = scene.bounds(&modal_path(&[2, 0, 0]));
    let save = scene.bounds(&modal_path(&[2, 0, 1]));
    let save_label = scene.bounds(&modal_path(&[2, 0, 1, 0]));
    assert!((save.center().y - save_label.center().y).abs() < 0.5);
    assert!(field.width >= 280.0 && error.y < cancel.y && save.y + save.height < 440.0);
    assert!(scene.click(&mut renderer, field.center()).is_empty());
    assert!(matches!(
        scene.send(&mut renderer, &[ime("fixture-虛構-key")], field.center()).as_slice(),
        [Message::Settings(SettingsMessage::ProviderConnectApiKeyChanged(value))]
            if value == "fixture-虛構-key"
    ));
    assert!(
        scene
            .send(&mut renderer, &[key(Named::Enter), key(Named::Escape)], field.center())
            .is_empty()
    );
    assert!(matches!(
        scene.click(&mut renderer, save.center()).as_slice(),
        [Message::Settings(SettingsMessage::ProviderConnectSubmit)]
    ));
    assert!(matches!(
        scene.click(&mut renderer, cancel.center()).as_slice(),
        [Message::Settings(SettingsMessage::ProviderConnectClose)]
    ));
    drop(scene);
    for (locale, theme, name) in [
        (DesktopLocale::SimplifiedChinese, Theme::Light, "connect-failed-zh"),
        (DesktopLocale::TraditionalChinese, Theme::Dark, "connect-failed-zh-hant"),
    ] {
        app.desktop_locale = locale;
        capture(
            connect::view_overlays(&app, connected::view(&app)),
            &mut renderer,
            &theme,
            Size::new(420, 440),
            name,
        );
    }

    app.provider_settings.connect_modal = None;
    app.desktop_locale = DesktopLocale::English;
    app.provider_settings.models_syncing = true;
    app.provider_settings.models_sync_label = "Fixture sync".into();
    app.provider_settings.models_sync_progress = 0.42;
    capture(
        connected::view(&app),
        &mut renderer,
        &Theme::Dark,
        Size::new(640, 980),
        "providers-busy",
    );
    let mut scene = Scene::new(connected::view(&app), &mut renderer, Size::new(640.0, 980.0));
    for path in [[0, 1], [0, 2]] {
        assert!(scene.click(&mut renderer, scene.bounds(&path).center()).is_empty());
    }
    drop(scene);

    app.provider_settings.catalog_open = true;
    app.provider_settings.catalog_query = "fixture-00".into();
    let mut scene = Scene::new(
        catalog::view_overlays(&app, background()),
        &mut renderer,
        Size::new(420.0, 620.0),
    );
    let actions: &[usize] = &[2, 0, 0, 0, 0, 0, 0, 1, 0];
    for index in [0, 1] {
        let point = scene.bounds(&modal_path(&[actions, &[index]].concat())).center();
        assert!(scene.click(&mut renderer, point).is_empty());
    }
    drop(scene);

    app.provider_settings.catalog_query = "nonexistent-fixture".into();
    capture(
        catalog::view_overlays(&app, background()),
        &mut renderer,
        &Theme::Light,
        Size::new(420, 620),
        "catalog-no-results",
    );
    app.provider_settings.catalog_query.clear();
    app.provider_settings.catalog_items.clear();
    app.provider_settings.providers.clear();
    capture(
        catalog::view_overlays(&app, background()),
        &mut renderer,
        &Theme::Light,
        Size::new(420, 620),
        "catalog-empty",
    );
    capture(
        connected::view(&app),
        &mut renderer,
        &Theme::Light,
        Size::new(640, 980),
        "providers-empty",
    );
    app.provider_settings.catalog_loading = true;
    capture(
        catalog::view_overlays(&app, background()),
        &mut renderer,
        &Theme::Dark,
        Size::new(420, 620),
        "catalog-loading",
    );
    app.provider_settings.catalog_loading = false;
    app.provider_settings.save_error = Some("Fixture catalog failure".into());
    capture(
        catalog::view_overlays(&app, background()),
        &mut renderer,
        &Theme::Dark,
        Size::new(420, 620),
        "catalog-failed",
    );
}

fn background() -> Element<'static, Message> {
    iced::widget::button("Underlying settings")
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .on_press(Message::Settings(SettingsMessage::ProvidersRefresh))
        .into()
}

// Stack → active layer → centering container → inset → card → contents.
fn modal_path(path: &[usize]) -> Vec<usize> {
    [&[1, 1, 0, 0, 0][..], path].concat()
}

struct Scene<'a> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    layout: layout::Node,
}

impl<'a> Scene<'a> {
    fn new(mut element: Element<'a, Message>, renderer: &mut Renderer, size: Size) -> Self {
        let mut tree = Tree::new(element.as_widget());
        let layout = element.as_widget_mut().layout(
            &mut tree,
            renderer,
            &layout::Limits::new(Size::ZERO, size),
        );
        let ui = UserInterface::build(element, size, Cache::new(), renderer);
        Self { ui, layout }
    }

    fn bounds(&self, path: &[usize]) -> Rectangle {
        path.iter()
            .fold(Layout::new(&self.layout), |layout, &index| {
                layout
                    .children()
                    .nth(index)
                    .unwrap_or_else(|| panic!("missing layout path {path:?}"))
            })
            .bounds()
    }

    fn send(&mut self, renderer: &mut Renderer, events: &[Event], point: Point) -> Vec<Message> {
        let mut messages = Vec::new();
        self.ui.update(
            events,
            mouse::Cursor::Available(point),
            renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        messages
    }

    fn click(&mut self, renderer: &mut Renderer, point: Point) -> Vec<Message> {
        self.send(
            renderer,
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            point,
        )
    }

    fn draw(&mut self, renderer: &mut Renderer, theme: &Theme, cursor: mouse::Cursor) {
        self.ui.draw(renderer, theme, &Style { text_color: theme.palette().text }, cursor);
    }
}

#[derive(Default)]
struct ScrollOffset(f32);

impl Operation for ScrollOffset {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        _bounds: Rectangle,
        _content: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.0 = translation.y;
    }
}

fn key(key: Named) -> Event {
    let key = Key::Named(key);
    Event::Keyboard(keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    })
}

fn ime(text: &str) -> Event {
    Event::InputMethod(iced::advanced::input_method::Event::Commit(text.into()))
}
