//! 验证提供商连接弹窗在关闭、错误和多种输入状态下的布局。
//!
//! 连接夹具覆盖空值、Unicode 与错误文本，树节点数量用于确认浮层层级不漂移。

use crate::app::state::ProviderConnectState;
use crate::app::{App, Message};
use iced::advanced::widget::Tree;
use iced::widget::{container, text};
use iced::{Element, Length};

fn test_app() -> App {
    App::new().0
}

#[test]
fn shiju_model_key_dialog_remains_hidden_even_with_stale_modal_state() {
    let mut app = test_app();
    app.provider_settings.connect_modal = Some(ProviderConnectState {
        provider_id: "shiju".to_string(),
        provider_name: "识矩".to_string(),
        api_key: "issued-model-secret".to_string(),
    });
    let base: Element<'_, Message> = container(text("base")).width(Length::Fixed(321.0)).into();
    let element = super::connect::view_overlays(&app, base);
    assert_eq!(element.as_widget().size().width, Length::Fixed(321.0));
    assert!(element.as_widget().children().is_empty());
}

fn connect_app(provider_name: &str, api_key: &str, error: Option<&str>) -> App {
    let mut app = test_app();
    app.provider_settings.connect_modal = Some(ProviderConnectState {
        provider_id: "provider-id".to_string(),
        provider_name: provider_name.to_string(),
        api_key: api_key.to_string(),
    });
    app.provider_settings.connect_error = error.map(str::to_string);
    app
}

fn tree_node_count(tree: &Tree) -> usize {
    1 + tree.children.iter().map(tree_node_count).sum::<usize>()
}

fn element_node_count(element: &Element<'_, Message>) -> usize {
    1 + element.as_widget().children().iter().map(tree_node_count).sum::<usize>()
}

#[test]
fn view_overlays_returns_original_base_dimensions_when_modal_is_closed() {
    let app = test_app();
    let base: Element<'_, Message> =
        container(text("dialog")).width(Length::Fixed(321.0)).height(Length::Fixed(123.0)).into();

    let element = super::connect::view_overlays(&app, base);

    assert_eq!(element.as_widget().size().width, Length::Fixed(321.0));
    assert_eq!(element.as_widget().size().height, Length::Fixed(123.0));
    assert!(element.as_widget().children().is_empty());
}

#[test]
fn view_overlays_ignores_stale_error_when_modal_is_closed() {
    let mut app = test_app();
    app.provider_settings.connect_error = Some("连接失败".to_string());
    let base: Element<'_, Message> =
        container(text("dialog")).width(Length::Fixed(280.0)).height(Length::Fixed(90.0)).into();

    let element = super::connect::view_overlays(&app, base);

    assert_eq!(element.as_widget().size().width, Length::Fixed(280.0));
    assert_eq!(element.as_widget().size().height, Length::Fixed(90.0));
}

#[test]
fn view_overlays_builds_fill_overlay_with_base_and_modal_layers() {
    let app = connect_app("OpenAI", "test-key", None);

    let element = super::connect::view_overlays(&app, text("dialog").into());

    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn view_overlays_adds_error_banner_when_error_is_present() {
    let without_error = connect_app("OpenAI", "test-key", None);
    let with_error = connect_app("OpenAI", "test-key", Some("连接失败"));

    let clean = super::connect::view_overlays(&without_error, text("dialog").into());
    let errored = super::connect::view_overlays(&with_error, text("dialog").into());

    assert!(element_node_count(&errored) > element_node_count(&clean));
    assert_eq!(errored.as_widget().children().len(), clean.as_widget().children().len());
}

#[test]
fn view_overlays_treats_empty_error_as_present_banner() {
    let without_error = connect_app("OpenAI", "", None);
    let with_empty_error = connect_app("OpenAI", "", Some(""));

    let clean = super::connect::view_overlays(&without_error, text("dialog").into());
    let errored = super::connect::view_overlays(&with_empty_error, text("dialog").into());

    assert!(element_node_count(&errored) > element_node_count(&clean));
}

#[test]
fn view_overlays_keeps_structure_for_empty_and_unicode_fields() {
    let empty = connect_app("", "", None);
    let unicode = connect_app("本地 Provider 🚀", "密钥-🔐", None);

    let empty_element = super::connect::view_overlays(&empty, text("dialog").into());
    let unicode_element = super::connect::view_overlays(&unicode, text("dialog").into());

    assert_eq!(element_node_count(&empty_element), element_node_count(&unicode_element));
    assert_eq!(empty_element.as_widget().size(), unicode_element.as_widget().size());
}

#[test]
fn view_overlays_keeps_fill_modal_when_empty_credentials_have_an_error() {
    let app = connect_app("", "", Some("缺少 API 密钥"));

    let element = super::connect::view_overlays(&app, text("dialog").into());

    assert_eq!(element.as_widget().children().len(), 2);
}
