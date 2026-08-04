use super::*;
use crate::app::message::settings::{MultimodalMessage, SettingsMessage};
use crate::app::state::{ModelSummary, ProviderModelsSummary};
use crate::app::{App, Message};
use iced::Element;
use iced::Length;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::widget::text;
use iced::{Color, Font, Pixels, Theme};
use serde_json::json;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn model(id: &str, enabled: bool, image_input: serde_json::Value) -> ModelSummary {
    ModelSummary {
        id: id.to_string(),
        name: format!("动态模型 {id}"),
        enabled,
        toolcall: false,
        attachment: false,
        context_limit: 8_192,
        detail: json!({
            "capabilities": {
                "input": {
                    "image": image_input,
                },
            },
        }),
    }
}

fn provider(id: &str, models: Vec<ModelSummary>) -> ProviderModelsSummary {
    ProviderModelsSummary { id: id.to_string(), name: format!("提供者 {id}"), models }
}

#[test]
fn image_model_options_only_include_active_image_input_models() {
    let mut app = test_app();
    app.model_settings.providers = vec![
        provider(
            "openai",
            vec![
                model("vision", true, json!(true)),
                model("disabled-vision", false, json!(true)),
                model("text", true, json!(false)),
                model("unknown", true, serde_json::Value::Null),
            ],
        ),
        provider("", vec![model("missing-provider", true, json!(true))]),
        provider("google", vec![model("", true, json!(true))]),
        provider("google", vec![model("gemini-vision", true, json!(true))]),
    ];

    assert_eq!(
        image_model_options(&app),
        vec!["openai/vision".to_string(), "google/gemini-vision".to_string()]
    );
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
fn view_covers_loading_error_selected_and_empty_catalog_states() {
    let mut app = test_app();

    app.model_settings.loading = true;
    app.multimodal_settings.ocr_model = "openai/vision".to_string();
    keep_element(view(&app));

    app.model_settings.loading = false;
    app.model_settings.save_error = Some("模型目录不可用".to_string());
    keep_element(view(&app));

    app.model_settings.save_error = None;
    app.model_settings.providers =
        vec![provider("openai", vec![model("vision", true, json!(true))])];
    app.multimodal_settings.ocr_enabled = true;
    let element = view(&app);
    assert_eq!(element.as_widget().size().width, Length::Fill);
    assert_eq!(element.as_widget().children().len(), 5);
    drop(element);

    app.model_settings.providers.clear();
    keep_element(view(&app));
}

#[test]
fn multimodal_search_keywords_include_ocr_and_image_model() {
    use crate::app::components::system_settings::{SystemTab, system_tab_matches_query};

    assert!(system_tab_matches_query(SystemTab::Multimodal, "OCR"));
    assert!(system_tab_matches_query(SystemTab::Multimodal, "图片模型"));
}

#[tokio::test(flavor = "multi_thread")]
async fn multimodal_view_draws_non_ascii_dynamic_model_in_dark_theme() {
    let mut app = test_app();
    app.model_settings.providers =
        vec![provider("提供者", vec![model("视觉模型", true, json!(true))])];
    app.multimodal_settings.ocr_model = "提供者/视觉模型".to_string();
    app.multimodal_settings.ocr_enabled = true;

    let mut element = view(&app);
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let viewport = iced::Rectangle::with_size(iced::Size::new(960.0, 1_200.0));
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &iced::advanced::layout::Limits::new(iced::Size::ZERO, viewport.size()),
    );

    element.as_widget().draw(
        &tree,
        &mut renderer,
        &Theme::Dark,
        &iced::advanced::renderer::Style::default(),
        iced::advanced::Layout::new(&node),
        iced::mouse::Cursor::Unavailable,
        &viewport,
    );

    let pixels = renderer.screenshot(iced::Size::new(960, 1_200), 1.0, Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
}
