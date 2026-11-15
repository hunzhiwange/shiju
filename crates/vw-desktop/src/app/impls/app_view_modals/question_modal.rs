use super::{App, Message, message};
use iced::Element;
pub(crate) fn with_question_modal<'a>(app: &App, root: Element<'a, Message>) -> Element<'a, Message> {
    desktop_chat_view::modals::question_modal::with_question_modal(app.question_modal_request.as_ref(), &app.question_modal_answers, &app.question_modal_custom, app.desktop_locale, app.active_toast.as_ref().map(|_| crate::app::components::toast::view(app)), |event| {
        use desktop_chat_view::modals::question_modal::Event;
        Message::Chat(match event {
            Event::QuestionReject => message::ChatMessage::QuestionReject,
            Event::QuestionSubmit => message::ChatMessage::QuestionSubmit,
            Event::QuestionOptionToggled(index, label) => message::ChatMessage::QuestionOptionToggled(index, label),
            Event::QuestionCustomChanged(index, value) => message::ChatMessage::QuestionCustomChanged(index, value),
        })
    }, root)
}

#[cfg(test)]
#[path = "question_modal_tests.rs"]
mod question_modal_tests;
