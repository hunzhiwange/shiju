pub fn queue_panel(
    i18n: crate::app::i18n::I18n,
    queue: &[crate::app::QueueItem],
    requesting: bool,
) -> iced::Element<'static, crate::app::Message> {
    let queue = queue
        .iter()
        .map(|q| desktop_chat_input::input_panel::queue_panel::QueuePreview {
            query: q.query.clone(),
            is_guide: q.send_behavior == crate::app::state::ChatSendBehavior::Guide,
            attachments: q
                .attachments
                .iter()
                .cloned()
                .map(super::attachment::AttachmentDisplayItem::from_local_path)
                .collect(),
        })
        .collect();
    desktop_chat_input::input_panel::queue_panel::queue_panel(i18n, queue, requesting)
        .map(super::slice_message)
}
