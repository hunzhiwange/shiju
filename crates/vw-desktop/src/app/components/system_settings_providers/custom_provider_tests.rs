//! 验证自定义提供商弹窗的创建、编辑、请求头和模型列表状态。
//!
//! 请求头和模型集合包含空集与多项数据，用于确认动态编辑行按配置数量生成。

use crate::app::state::{CustomProviderModelDraft, ProviderHeaderDraft};
use crate::app::{App, Message, message};
use iced::advanced::widget::Tree;
use iced::widget::text;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn element_tree(element: iced::Element<'_, Message>) -> Tree {
    Tree::new(element.as_widget())
}

fn tree_node_count(tree: &Tree) -> usize {
    1 + tree.children.iter().map(tree_node_count).sum::<usize>()
}

#[test]
fn overlays_return_base_when_modal_is_closed() {
    let app = test_app();

    keep_element(super::custom_provider::view_overlays(&app, text("dialog").into()));
}

#[test]
fn overlays_build_create_modal_with_single_and_empty_collections() {
    let mut app = test_app();

    app.provider_settings.custom_provider_modal_open = true;
    app.provider_settings.custom.provider_id = "local-ai".to_string();
    app.provider_settings.custom.display_name = "Local AI".to_string();
    app.provider_settings.custom.base_url = "http://localhost:11434/v1".to_string();
    app.provider_settings.custom.api_key = "sk-local".to_string();
    keep_element(super::custom_provider::view_overlays(&app, text("dialog").into()));

    app.provider_settings.custom.headers.clear();
    app.provider_settings.custom.models.clear();
    keep_element(super::custom_provider::view_overlays(&app, text("dialog").into()));
}

#[test]
fn helper_rows_build_label_plain_input_and_secure_input_controls() {
    keep_element(super::custom_provider::field_row("标签", "说明", text("control")));
    keep_element(super::custom_provider::text_row(
        "显示名称",
        "展示给用户的 provider 名称。",
        "我是 AI 提供商",
        "Local AI",
        |v| Message::Settings(message::SettingsMessage::CustomProviderNameChanged(v)),
    ));
    keep_element(super::custom_provider::secure_text_row(
        "API Key",
        "可选默认密钥；填写后会作为 Authorization 头使用。",
        "sk-...",
        "test-key",
        |v| Message::Settings(message::SettingsMessage::CustomProviderApiKeyChanged(v)),
    ));
}

#[test]
fn overlays_build_edit_modal_headers_models_and_error_states() {
    let mut app = test_app();
    app.provider_settings.custom_provider_modal_open = true;
    app.provider_settings.custom_editing_provider_id = Some("local-ai".to_string());
    app.provider_settings.custom.provider_id = "local-ai".to_string();
    app.provider_settings.custom.display_name = "Local AI".to_string();
    app.provider_settings.custom.base_url = "http://localhost:11434/v1".to_string();
    app.provider_settings.custom.api_key = "sk-local".to_string();
    app.provider_settings.custom.headers = vec![
        ProviderHeaderDraft { key: "x-route".to_string(), value: "local".to_string() },
        ProviderHeaderDraft { key: "x-team".to_string(), value: "desktop".to_string() },
    ];
    app.provider_settings.custom.models = vec![
        CustomProviderModelDraft {
            model_id: "llama3.1".to_string(),
            display_name: "Llama 3.1".to_string(),
            enabled: true,
            ..Default::default()
        },
        CustomProviderModelDraft {
            model_id: "qwen".to_string(),
            display_name: String::new(),
            enabled: false,
            from_catalog: true,
            ..Default::default()
        },
        CustomProviderModelDraft {
            model_id: String::new(),
            display_name: String::new(),
            ..Default::default()
        },
    ];
    app.provider_settings.custom_provider_tab = crate::app::state::CustomProviderTab::Models;
    keep_element(super::custom_provider::view_overlays(&app, text("dialog").into()));

    app.provider_settings.custom_models_filter = crate::app::state::CustomProviderModelFilter::All;
    keep_element(super::custom_provider::view_overlays(&app, text("dialog").into()));

    app.provider_settings.custom_models_filter =
        crate::app::state::CustomProviderModelFilter::Disabled;
    keep_element(super::custom_provider::view_overlays(&app, text("dialog").into()));

    app.provider_settings.save_error = Some("保存失败".to_string());
    keep_element(super::custom_provider::view_overlays(&app, text("dialog").into()));

    app.provider_settings.custom_models_discovering = true;
    keep_element(super::custom_provider::view_overlays(&app, text("dialog").into()));
}

#[test]
fn overlays_adds_one_editable_row_per_configured_header() {
    let mut app = test_app();
    app.provider_settings.custom_provider_modal_open = true;
    app.provider_settings.custom_provider_tab = crate::app::state::CustomProviderTab::Authentication;
    let single_header =
        element_tree(super::custom_provider::view_overlays(&app, text("dialog").into()));

    app.provider_settings
        .custom
        .headers
        .push(ProviderHeaderDraft { key: "x-team".to_string(), value: "desktop".to_string() });
    let two_headers =
        element_tree(super::custom_provider::view_overlays(&app, text("dialog").into()));

    assert_eq!(single_header.children.len(), 2);
    assert_eq!(two_headers.children.len(), 2);
    assert!(tree_node_count(&two_headers) > tree_node_count(&single_header));
}
