//! Localized shared help preserves the overlay in both copy states.
use desktop_ui_controls::system_settings_common::help::localized_settings_help_modal;
#[derive(Clone)]
enum Message {
    None,
    GatewayHealthTick,
}
use iced::widget::text;

#[test]
fn help_modal_fills_overlay_for_default_and_copied_states() {
    let mut last_copied_code_hash = None;
    let help_text = String::from("帮助内容");

    let default = localized_settings_help_modal(
        Default::default(),
        last_copied_code_hash,
        text("base").into(),
        "帮助",
        help_text.clone(),
        |_| Message::None,
        Message::GatewayHealthTick,
    );
    assert_eq!(default.as_widget().children().len(), 2);

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&help_text, &mut hasher);
    last_copied_code_hash = Some(std::hash::Hasher::finish(&hasher));

    let copied = localized_settings_help_modal(
        Default::default(),
        last_copied_code_hash,
        text("base").into(),
        "",
        help_text,
        |_| Message::None,
        Message::None,
    );
    assert_eq!(copied.as_widget().children().len(), 2);
}
