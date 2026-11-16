//! Desktop cross-domain ports for the shared tool renderer.
use crate::app::{App, Message};
pub(crate) fn render_shared_tool_view<'a>(
    app: &'a App,
    m: usize,
    t: usize,
    raw: &str,
) -> Option<iced::Element<'a, Message>> {
    let input = desktop_chat_tools::tools::tool_renderer::ViewInput {
        card: super::card_adapter::input(app),
        result: desktop_chat_tools::tools::result_input::ResultInput {
            i18n: crate::app::i18n::I18n::new(app.desktop_locale),
            hovered: app.chat_tool_hovered_idx,
            question_request: app.question_modal_request.as_ref(),
            message_ids: &app.chat_message_ids,
        },
        file_filter: &app.tool_files_filter,
        file_hovered: app.chat_tool_file_hovered.as_deref(),
    };
    desktop_chat_tools::tools::tool_renderer::view(
        &input,
        m,
        t,
        raw,
        &|title, path, before, after| {
            super::git_diff_view::render_git_diff_content(app, title, path, before, after)
        },
        || crate::app::components::input_panel::todo_panel::read_todos_for_panel(app),
        super::card_adapter::map,
        super::card_adapter::map_card,
        super::advanced_view::map_message,
    )
}
