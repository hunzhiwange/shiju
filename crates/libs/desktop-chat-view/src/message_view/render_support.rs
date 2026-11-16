//! Compose local message widgets and map their events without a host dependency.
use super::markdown_selection::SelectionMessage;
use super::render_snapshot::{Action, MessageViewSnapshot};
use desktop_chat_tools::tool_text_support::ToolTextTarget;
use desktop_i18n::I18n;
use iced::{Color, Element};
pub fn message_text_body<'a, M: Clone + 'static>(
    app: &MessageViewSnapshot<'a, M>,
    content: String,
    user: bool,
) -> Element<'a, M> {
    let emit = app.emit.clone();
    super::text::message_text_body(content, user).map(move |event| emit(Action::Text(event)))
}
pub fn message_editor_body<'a, M: Clone + 'static>(
    app: &MessageViewSnapshot<'a, M>,
    idx: usize,
    color: Color,
) -> Option<Element<'a, M>> {
    let emit = app.emit.clone();
    super::text::message_editor_body(app.text.chat_message_editors, idx, color)
        .map(|v| v.map(move |event| emit(Action::Text(event))))
}
pub use message_editor_body as user_message_editor_body;
pub fn session_control_selection_card<'a, M: Clone + 'static>(
    app: &MessageViewSnapshot<'a, M>,
    i18n: I18n,
    selection: super::text::SessionControlSelection,
) -> Element<'a, M> {
    let emit = app.emit.clone();
    super::text::session_control_selection_card(i18n, selection)
        .map(move |event| emit(Action::Text(event)))
}
pub fn think_block_view_with_streaming<'a, M: Clone + 'static>(
    app: &MessageViewSnapshot<'a, M>,
    msg: usize,
    idx: usize,
    content: String,
    open: bool,
    timing: Option<&'a session_ui_types::ThinkTiming>,
    streaming: bool,
) -> Element<'a, M> {
    let emit = app.emit.clone();
    super::think_block::think_block_view_with_streaming(
        super::think_block::ThinkBlockState {
            show_reasoning_summary: app.dialogue_flow_show_reasoning_summary,
            expanded: app.chat_think_expanded,
            collapsed: app.chat_think_collapsed,
            hovered: app.chat_think_hovered_idx,
            editors: app.text.chat_think_editors,
            i18n: I18n::new(app.desktop_locale),
            now_ms: app.now_ms,
            animation_frame: app.status_animation_frame,
        },
        msg,
        idx,
        content,
        open,
        timing,
        streaming,
    )
    .map(move |event| emit(Action::Think(event)))
}
pub fn tool_text_editor<'a, M>(
    app: &MessageViewSnapshot<'a, M>,
    target: ToolTextTarget,
    _: &'static str,
    _: f32,
    _: bool,
    danger: bool,
) -> Option<Element<'a, M>> {
    (app.tools.editor)(target, danger)
}
pub fn render_shared_tool_view<'a, M>(
    app: &MessageViewSnapshot<'a, M>,
    msg: usize,
    tool: usize,
    raw: &str,
) -> Option<Element<'a, M>> {
    (app.tools.card)(msg, tool, raw)
}
pub fn tool_command_summary_view<'a, M>(
    app: &MessageViewSnapshot<'a, M>,
    msg: usize,
    items: &[desktop_chat_tools::tools::types::CommandItem],
) -> Option<Element<'a, M>> {
    (app.tools.commands)(msg, items)
}
pub fn tool_explore_summary_view<'a, M>(
    app: &MessageViewSnapshot<'a, M>,
    msg: usize,
    group: usize,
    items: &[desktop_chat_tools::tools::types::ExploreItem],
    running: bool,
    closed: bool,
) -> Option<Element<'a, M>> {
    (app.tools.explore)(msg, group, items, running, closed)
}
pub fn markdown_body_view<'a, M: SelectionMessage + 'static>(
    app: &MessageViewSnapshot<'a, M>,
    msg: usize,
    idx: usize,
    blocks: &'a [super::markdown::MarkdownBlock],
) -> Element<'a, M> {
    let emit = app.emit.clone();
    let copy_emit = emit.clone();
    let locale = app.desktop_locale;
    let copied_hash = app.last_copied_code_hash;
    let recent = app.copied_recently;
    let view = super::markdown_body::MarkdownView {
        theme: app.theme(),
        editors: app.text.chat_markdown_block_editors,
        session_id: app.active_session_id,
        selected: app.text.markdown_selection.is_some_and(|(message, _)| message == msg)
            && app.markdown_selection_text_idx == Some(idx),
        links: app.links.clone(),
        editor_action: std::rc::Rc::new(move |m, t, b, a| emit(Action::MarkdownEditor(m, t, b, a))),
        code_block: std::rc::Rc::new(move |language, code, body| {
            let emit = copy_emit.clone();
            let text = code.to_owned();
            super::markdown_code::code_block_view(
                I18n::new(locale),
                recent && copied_hash == Some(super::parse::hash_chat_content(code)),
                move || emit(Action::CopyCode(text.clone())),
                language,
                code,
                body,
            )
        }),
    };
    super::markdown_body::markdown_body_view(&view, msg, idx, blocks)
}
