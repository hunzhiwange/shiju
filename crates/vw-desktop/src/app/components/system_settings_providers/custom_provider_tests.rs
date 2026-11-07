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

    app.provider_settings.save_error = Some(("保存失败".to_string()).into());
    keep_element(super::custom_provider::view_overlays(&app, text("dialog").into()));

    app.provider_settings.custom_models_discovering = true;
    keep_element(super::custom_provider::view_overlays(&app, text("dialog").into()));
}

#[test]
fn overlays_adds_one_editable_row_per_configured_header() {
    let mut app = test_app();
    app.provider_settings.custom_provider_modal_open = true;
    app.provider_settings.custom_provider_tab =
        crate::app::state::CustomProviderTab::Authentication;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn settings_tabs_editors_appearance_provider() {
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::i18n::settings_models::connection_text::text as tr;
    use crate::app::message::SettingsMessage;
    use crate::app::state::CustomProviderTab;

    fn overlay(app: &App) -> iced::Element<'_, Message> {
        let background =
            iced::widget::Space::new().width(iced::Length::Fill).height(iced::Length::Fill);
        super::custom_provider::view_overlays(app, background.into())
    }

    tab_appearance_tests::run_at_least(
        concat!(module_path!(), "::settings_tabs_editors_appearance_provider"),
        420,
        test_app,
        |scene, app| {
            let _ = app.update(Message::Settings(SettingsMessage::CustomProviderOpen));
            let name = "自定义草稿 · Provider draft";
            let _ = app
                .update(Message::Settings(SettingsMessage::CustomProviderNameChanged(name.into())));
            app.provider_settings.custom.headers = (0..8)
                .map(|i| ProviderHeaderDraft {
                    key: format!("x-fixture-{i}"),
                    value: "draft".into(),
                })
                .collect();
            let tr = |label| tr(app.desktop_locale, label);
            let [basic, auth, models, title, save, cancel] =
                ["基础信息", "认证与请求头", "模型", "自定义提供商", "保存", "取消"].map(tr);
            scene.save_preview(app, overlay(app), "provider");
            let message = scene.check(app, overlay(app), &[basic, auth, models], 1);
            assert!(matches!(
                message,
                SettingsMessage::CustomProviderTabSelected(CustomProviderTab::Authentication)
            ));
            let _ = app.update(Message::Settings(message));
            scene.check_modal_scroll(overlay(app), title, save);

            app.provider_settings.custom_models_discovering = true;
            for (label, tab) in
                [(models, CustomProviderTab::Models), (basic, CustomProviderTab::Basic)]
            {
                let message = scene.click(overlay(app), label);
                let _ = app.update(Message::Settings(message));
                assert_eq!(app.provider_settings.custom_provider_tab, tab);
                scene.check_disabled(overlay(app), save);
                assert_eq!(app.provider_settings.custom.display_name, name);
                assert_eq!(app.provider_settings.custom.headers.len(), 8);
                assert!(app.provider_settings.custom_models_discovering);
            }
            let message = scene.click(overlay(app), cancel);
            assert!(matches!(message, SettingsMessage::CustomProviderClose));
            let _ = app.update(Message::Settings(message));
            assert!(!app.provider_settings.custom_provider_modal_open);
        },
    )
    .await;
}
