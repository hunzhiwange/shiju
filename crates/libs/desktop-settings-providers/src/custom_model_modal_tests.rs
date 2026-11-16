//! 模型弹窗使用局部提供商状态，无需启动桌面 App。
use crate::provider_view_test_support::{Fixture, style};
use crate::state::CustomProviderModelModalState;
use crate::view_message::Message;
use iced::advanced::widget::Tree;
use iced::widget::text;
fn overlays(app: &Fixture) -> iced::Element<'_, Message> {
    super::view_overlays(
        app.provider_settings.custom_model_modal.as_ref(),
        app.provider_settings.save_error.as_ref(),
        style(app),
        text("dialog").into(),
        |message| message,
    )
}
fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}
fn element_tree(element: iced::Element<'_, Message>) -> Tree {
    Tree::new(element.as_widget())
}
#[test]
fn overlays_return_base_add_modal_and_edit_modal() {
    let mut app = crate::provider_view_test_support::Fixture::new().0;
    keep_element(overlays(&app));

    app.provider_settings.custom_model_modal = Some(CustomProviderModelModalState {
        edit_index: None,
        model_id: "gpt-5".to_string(),
        display_name: "GPT-5".to_string(),
        ..Default::default()
    });
    keep_element(overlays(&app));

    app.provider_settings.custom_model_modal.as_mut().expect("modal").edit_index = Some(0);
    keep_element(overlays(&app));
}

#[test]
fn overlays_wrap_base_only_when_model_draft_exists() {
    let mut app = crate::provider_view_test_support::Fixture::new().0;
    let closed = element_tree(overlays(&app));

    app.provider_settings.custom_model_modal = Some(CustomProviderModelModalState {
        edit_index: None,
        model_id: String::new(),
        display_name: String::new(),
        ..Default::default()
    });
    let open = element_tree(overlays(&app));

    assert!(closed.children.is_empty());
    assert_eq!(open.children.len(), 2);
}
