pub fn queue_panel(
    i18n: crate::app::i18n::I18n,
    queue: Vec<crate::app::QueueItem>,
    requesting: bool,
) -> iced::Element<'static, crate::app::Message> {
    let queue = queue
        .into_iter()
        .map(|q| desktop_chat_input::input_panel::queue_panel::QueuePreview {
            query: q.query,
            is_guide: q.send_behavior == crate::app::state::ChatSendBehavior::Guide,
        })
        .collect();
    desktop_chat_input::input_panel::queue_panel::queue_panel(i18n, queue, requesting)
        .map(super::slice_message)
}
