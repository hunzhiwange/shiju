use super::*;
use crate::app::message::settings::{MultimodalMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::Element;
use iced::Length;
use iced::widget::text;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn field_row_builds_with_empty_and_populated_copy() {
    keep_element(field_row("", "", text("")));
    keep_element(field_row("最大图片数量", "单次请求最多允许上传的图片数。", text("4 张")));
}

#[test]
fn number_row_builds_plain_and_suffixed_boundary_values() {
    keep_element(number_row("最小值", "无单位", 0, 0, 0, "", |value| {
        Message::Settings(SettingsMessage::Multimodal(MultimodalMessage::MaxImagesChanged(value)))
    }));
    keep_element(number_row("最大值", "带单位", u32::MAX, 0, u32::MAX, "MB", |value| {
        Message::Settings(SettingsMessage::Multimodal(MultimodalMessage::MaxImageSizeMbChanged(
            value,
        )))
    }));
}

#[test]
fn bool_row_builds_disabled_and_enabled_remote_fetch_controls() {
    for checked in [false, true] {
        keep_element(bool_row(
            "允许远程抓取",
            "仅允许 http/https。",
            checked,
            "允许抓取",
            |value| {
                Message::Settings(SettingsMessage::Multimodal(
                    MultimodalMessage::AllowRemoteFetchToggled(value),
                ))
            },
        ));
    }
}

#[test]
fn view_builds_default_and_limit_settings() {
    let mut app = test_app();
    keep_element(view(&app));

    app.multimodal_settings.max_images = 16;
    app.multimodal_settings.max_image_size_mb = 20;
    app.multimodal_settings.allow_remote_fetch = true;
    keep_element(view(&app));

    app.multimodal_settings.max_images = 1;
    app.multimodal_settings.max_image_size_mb = 1;
    app.multimodal_settings.allow_remote_fetch = false;
    keep_element(view(&app));
}

#[test]
fn view_keeps_three_settings_sections_for_remote_fetch_states() {
    let mut app = test_app();

    for allow_remote_fetch in [false, true] {
        app.multimodal_settings.allow_remote_fetch = allow_remote_fetch;
        let element = view(&app);

        assert_eq!(element.as_widget().size().width, Length::Fill);
        assert_eq!(element.as_widget().children().len(), 3);
    }
}
