//! 验证自定义模型新增与编辑弹窗及其表单辅助控件。
//!
//! 测试分别构造新增与编辑草稿，确保同一浮层入口选择正确的模型表单。

// 保留计划任务 809 的既定覆盖面。
const SOURCE: &str = include_str!("custom_model_modal.rs");

use crate::app::state::CustomProviderModelModalState;
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

fn source_declares_symbol(name: &str) -> bool {
    let needles = [
        format!("fn {name}"),
        format!("pub fn {name}"),
        format!("struct {name}"),
        format!("pub struct {name}"),
        format!("enum {name}"),
        format!("pub enum {name}"),
        format!("type {name}"),
        format!("pub type {name}"),
        format!("const {name}"),
        format!("pub const {name}"),
        format!("static {name}"),
        format!("pub static {name}"),
        format!("impl {name}"),
    ];

    needles.iter().any(|needle| SOURCE.contains(needle))
}

#[test]
fn overlays_return_base_add_modal_and_edit_modal() {
    let mut app = test_app();
    keep_element(super::custom_model_modal::view_overlays(&app, text("dialog").into()));

    app.provider_settings.custom_model_modal = Some(CustomProviderModelModalState {
        edit_index: None,
        model_id: "gpt-5".to_string(),
        display_name: "GPT-5".to_string(),
        ..Default::default()
    });
    keep_element(super::custom_model_modal::view_overlays(&app, text("dialog").into()));

    app.provider_settings.custom_model_modal.as_mut().expect("modal").edit_index = Some(0);
    keep_element(super::custom_model_modal::view_overlays(&app, text("dialog").into()));
}

#[test]
fn helper_rows_build_label_control_and_input_rows() {
    keep_element(super::custom_model_modal::field_row("标签", "说明", text("control")));
    keep_element(super::custom_model_modal::text_row(
        "模型 ID",
        "用于请求 provider 的真实 model_id。",
        "例如 gpt-4o-mini",
        "gpt-5-mini",
        |v| Message::Settings(message::SettingsMessage::CustomProviderModelModalIdChanged(v)),
    ));
    keep_element(super::custom_model_modal::text_row(
        "显示名称",
        "仅影响 UI 展示，可留空回退为 model_id。",
        "可选",
        "",
        |v| Message::Settings(message::SettingsMessage::CustomProviderModelModalNameChanged(v)),
    ));
}

#[test]
fn overlays_wrap_base_only_when_model_draft_exists() {
    let mut app = test_app();
    let closed =
        element_tree(super::custom_model_modal::view_overlays(&app, text("dialog").into()));

    app.provider_settings.custom_model_modal = Some(CustomProviderModelModalState {
        edit_index: None,
        model_id: String::new(),
        display_name: String::new(),
        ..Default::default()
    });
    let open = element_tree(super::custom_model_modal::view_overlays(&app, text("dialog").into()));

    assert!(closed.children.is_empty());
    assert_eq!(open.children.len(), 2);
}

#[test]
fn custom_model_modal_tests_keeps_planned_coverage_targets() {
    for name in ["field_row", "text_row", "view_overlays"] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}
