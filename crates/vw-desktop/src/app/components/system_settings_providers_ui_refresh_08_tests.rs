// 宿主契约：可选择帮助菜单的 CopyCode/OpenUrlExternal 映射，以及提交校验错误投影。
// 列表、目录、连接弹窗的局部渲染迁至 desktop_split_11_tests::providers。
use super::{connect, custom_provider};
use crate::app::message::SettingsMessage;
use crate::app::state::ProviderConnectState;
use crate::app::{App, Message};
use iced::keyboard::key::Named;
use iced::{Element, Event, Point, Size, Theme, mouse};
#[path = "../../../../libs/desktop-settings-providers/src/view_test_support.rs"]
mod rendering;
use config_ui::DesktopLocale;
use rendering::{Scene, ime, key, modal_path, renderer};

fn fixture() -> App {
    let mut app = None;
    crate::app::config::with_editor_settings_gateway(|fixture, _, _| app = Some(fixture));
    let mut app = app.expect("isolated settings fixture");
    app.general_settings.desktop_locale = DesktopLocale::English;
    rendering::seed_catalog(&mut app.provider_settings);
    app
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn help_rollout_05_keeps_credentials_and_validation_outside_help() {
    use crate::app::i18n::settings_models::connection_text::text as tr;
    use crate::app::message::ViewMessage;

    let mut renderer = renderer().await;
    let mut app = fixture();
    app.appearance.app_theme = Theme::Light;
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
    let description = tr(app.general_settings.desktop_locale, "录入并保存当前 Provider 的 API 密钥。");
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
        app.provider_settings.connect_error.as_ref().unwrap().display(app.general_settings.desktop_locale),
        tr(app.general_settings.desktop_locale, "请输入 API 密钥")
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
            app.provider_settings.save_error.as_ref().unwrap().display(app.general_settings.desktop_locale),
            tr(app.general_settings.desktop_locale, expected_error)
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

fn background() -> Element<'static, Message> {
    iced::widget::button("Underlying settings")
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .on_press(Message::Settings(SettingsMessage::ProvidersRefresh))
        .into()
}
