//! 宿主契约：last_copied_code_hash 经本地化帮助适配器投影为复制状态。
//! 桌面帮助弹窗与 App 复制状态的组合测试。
use crate::app::{App, Message};
use iced::widget::text;

#[test]
fn help_modal_fills_overlay_for_default_and_copied_states() {
    let mut app = App::new().0;
    let help_text = String::from("帮助内容");

    let default = super::help::with_settings_help_modal(
        &app,
        text("base").into(),
        "帮助",
        help_text.clone(),
        Message::GatewayHealthTick,
    );
    assert_eq!(default.as_widget().children().len(), 2);

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&help_text, &mut hasher);
    app.last_copied_code_hash = Some(std::hash::Hasher::finish(&hasher));

    let copied = super::help::with_settings_help_modal(
        &app,
        text("base").into(),
        "",
        help_text,
        Message::None,
    );
    assert_eq!(copied.as_widget().children().len(), 2);
}
