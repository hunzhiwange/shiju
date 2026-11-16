//! Tool cards use real domain renderers and the real Git diff injection.
use desktop_chat_tools::{card_input::CardInput, tool_text_support::ToolTextState};
use iced::{Element, widget::text_editor};
use std::collections::{HashMap, HashSet};
#[derive(Debug, Clone)]
pub enum Message {
    Chat(desktop_chat_tools::advanced_input::Message),
    Other,
}
pub mod message {
    pub use desktop_chat_tools::advanced_input::Message as ChatMessage;
}
#[derive(Default)]
pub struct ContextMenu {
    pub target: Option<u64>,
    pub position: Option<(f32, f32)>,
}
#[derive(Default)]
pub struct App {
    pub chat_tool_hovered_idx: Option<u64>,
    pub project_path: Option<String>,
    pub desktop_locale: config_ui::DesktopLocale,
    pub chat_tool_file_expanded: HashSet<String>,
    pub chat_tool_text_editors: HashMap<u128, text_editor::Content>,
    pub chat_context_menu: ContextMenu,
    expanded: HashSet<u64>,
    empty: HashMap<u64, text_editor::Content>,
    wide: HashMap<u128, text_editor::Content>,
    git: crate::test_support::Fixture,
}
impl App {
    pub fn new() -> (Self, ()) {
        (Self::default(), ())
    }
    fn card(&self) -> CardInput<'_> {
        CardInput {
            i18n: desktop_i18n::I18n::new(self.desktop_locale),
            hovered: self.chat_tool_hovered_idx,
            expanded: &self.expanded,
            file_expanded: &self.chat_tool_file_expanded,
            project_path: self.project_path.as_deref(),
            text: ToolTextState {
                chat_message_editors: &[],
                chat_special_text_editors: &self.empty,
                chat_tool_text_editors: &self.chat_tool_text_editors,
                chat_think_editors: &self.empty,
                chat_markdown_block_editors: &self.wide,
                markdown_selection: None,
                theme: iced::Theme::Light,
            },
        }
    }
    fn result(&self) -> desktop_chat_tools::tools::result_input::ResultInput<'_> {
        desktop_chat_tools::tools::result_input::ResultInput {
            i18n: desktop_i18n::I18n::new(self.desktop_locale),
            hovered: self.chat_tool_hovered_idx,
            question_request: None,
            message_ids: &[],
        }
    }
    fn diff(
        &self,
        title: String,
        file: Option<String>,
        before: String,
        after: String,
    ) -> Element<'_, Message> {
        let mut input = self.git.view();
        input.project_path = &self.project_path;
        crate::page::embedded_custom_text_diff_view(&input, title, file, before, after, None)
            .map(|_| Message::Other)
    }
}
pub use desktop_chat_tools::tool_text_support::tool_text_key;
pub use desktop_chat_tools::tools::types::CommandItem;
mod tool_meta {
    pub use desktop_chat_tools::tool_presentation::tool_visual_state;
    pub use desktop_chat_tools::tools::tool_meta::*;
}
mod advanced_view {
    use super::*;
    pub fn agent_tool_detail_message(m: usize, t: usize, raw: &str) -> Message {
        Message::Chat(desktop_chat_tools::tools::advanced_view::agent_tool_detail_message(
            m, t, raw,
        ))
    }
    pub fn tool_advanced_view<'a>(
        app: &'a App,
        m: usize,
        t: usize,
        raw: &str,
    ) -> Option<Element<'a, Message>> {
        let card = app.card();
        desktop_chat_tools::tools::advanced_view::tool_advanced_view(
            desktop_chat_tools::advanced_input::AdvancedInput {
                i18n: card.i18n,
                hovered: card.hovered,
                text: card.text,
            },
            m,
            t,
            raw,
        )
        .map(|v| v.map(Message::Chat))
    }
}
mod apply_patch_view {
    use super::*;
    pub use desktop_chat_tools::tools::apply_patch_view::apply_patch_file_expand_key;
    pub fn tool_apply_patch_view<'a>(
        app: &'a App,
        m: usize,
        t: usize,
        raw: &str,
    ) -> Option<Element<'a, Message>> {
        desktop_chat_tools::tools::apply_patch_view::tool_apply_patch_view(
            &app.card(),
            m,
            t,
            raw,
            |title, path, before, after| app.diff(title, path, before, after),
        )
        .map(|v| v.map(|_| Message::Other))
    }
}
mod git_diff_view {
    use super::*;
    pub fn tool_git_diff_view<'a>(
        app: &'a App,
        m: usize,
        t: usize,
        raw: &str,
    ) -> Option<Element<'a, Message>> {
        desktop_chat_tools::tools::git_diff_view::tool_git_diff_view(
            &app.result(),
            m,
            t,
            raw,
            &|title, path, before, after| app.diff(title, path, before, after),
        )
        .map(|v| v.map(|_| Message::Other))
    }
}

fn tool_skill_view<'a>(
    app: &'a App,
    m: usize,
    t: usize,
    raw: &str,
) -> Option<Element<'a, Message>> {
    desktop_chat_tools::tools::skill_view::tool_skill_view(&app.card(), m, t, raw)
        .map(|v| v.map(|_| Message::Other))
}

fn tool_web_view<'a>(app: &'a App, m: usize, t: usize, raw: &str) -> Option<Element<'a, Message>> {
    desktop_chat_tools::tools::web_view::tool_web_view(&app.card(), m, t, raw)
        .map(|v| v.map(|_| Message::Other))
}
fn tool_workflow_view<'a>(
    app: &'a App,
    m: usize,
    t: usize,
    raw: &str,
) -> Option<Element<'a, Message>> {
    desktop_chat_tools::tools::workflow_view::tool_workflow_view(&app.card(), m, t, raw)
        .map(|v| v.map(|_| Message::Other))
}
fn tool_todowrite_compact_view<'a>(
    app: &'a App,
    m: usize,
    t: usize,
    raw: &str,
) -> Option<Element<'a, Message>> {
    desktop_chat_tools::tools::todo_views::tool_todowrite_compact_view(&app.card(), m, t, raw)
        .map(|v| v.map(|_| Message::Other))
}
fn tool_mcp_view<'a>(app: &'a App, m: usize, t: usize, raw: &str) -> Option<Element<'a, Message>> {
    desktop_chat_tools::tools::mcp_view::tool_mcp_view(&app.result(), m, t, raw)
        .map(|v| v.map(|_| Message::Other))
}
fn tool_question_view<'a>(
    app: &'a App,
    m: usize,
    t: usize,
    raw: &str,
) -> Option<Element<'a, Message>> {
    desktop_chat_tools::tools::question_view::tool_question_view(&app.result(), m, t, raw)
        .map(|v| v.map(|_| Message::Other))
}

fn tool_command_summary_view<'a>(
    app: &'a App,
    m: usize,
    items: &[CommandItem],
) -> Option<Element<'a, Message>> {
    desktop_chat_tools::tools::command_summary_view::tool_command_summary_view(
        &app.card(),
        m,
        items,
    )
    .map(|v| v.map(|_| Message::Other))
}
mod mcp_view {
    pub(super) use super::tool_mcp_view;
}
mod question_view {
    pub(super) use super::tool_question_view;
}
mod skill_view {
    pub(super) use super::tool_skill_view;
}
mod web_view {
    pub(super) use super::tool_web_view;
}
mod workflow_view {
    pub(super) use super::tool_workflow_view;
}
mod todo_views {
    pub(super) use super::tool_todowrite_compact_view;
}
mod tool_renderer {
    use super::*;
    pub fn render_shared_tool_view<'a>(
        app: &'a App,
        m: usize,
        t: usize,
        raw: &str,
    ) -> Option<Element<'a, Message>> {
        let input = desktop_chat_tools::tools::tool_renderer::ViewInput {
            card: app.card(),
            result: app.result(),
            file_filter: "",
            file_hovered: None,
        };
        desktop_chat_tools::tools::tool_renderer::view(
            &input,
            m,
            t,
            raw,
            &|title, path, before, after| app.diff(title, path, before, after),
            || None,
            |_| Message::Other,
            |_| Message::Other,
            Message::Chat,
        )
    }
}
mod advanced_view_tests;
mod apply_patch_view_tests;
mod tool_renderer_tests;
