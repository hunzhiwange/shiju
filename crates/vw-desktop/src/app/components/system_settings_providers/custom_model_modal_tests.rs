//! 验证自定义模型新增与编辑弹窗。
//!
//! 测试分别构造新增与编辑草稿，确保同一浮层入口选择正确的模型表单。

use crate::app::state::CustomProviderModelModalState;
use crate::app::{App, Message};
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
