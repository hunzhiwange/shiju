//! Slash palette 查询检测与分组候选排序。
//!
//! 本模块负责识别 chat 输入尾部 `/query`，并按 Skills / Tools / MCP / Models / ACP
//! 分组构建可折叠的模糊排序结果，供状态机与 overlay UI 复用。

use super::shared::{focus_input_editor, sync_global_input_editor_if_needed};
use crate::app::state::tool_display_name;
use crate::app::{App, FocusArea, Message, message};
use iced::Task;

pub(crate) fn clear_slash_palette_state(app: &mut App) {
    app.input_suggestions.clear_slash();
}

pub(crate) fn collect_slash_palette_candidates(app: &App) -> SlashPaletteCandidates {
    let runtime = app.current_session_runtime();
    let skills = app.skills_settings.catalog.iter().map(|skill| SlashPaletteCandidate {
        id: skill.id.clone(), label: skill.title.clone(), description: Some(skill.description.clone()),
    }).collect();
    // 工具目录及智能体启动能力由其真实领域入口提供。
    let tools = if runtime.acp_agent.is_none() { app.session_tool_inventory(&runtime).base_tools.into_iter().map(|id| {
        let label = tool_display_name(&id);
        SlashPaletteCandidate { id, label, description: None }
    }).collect() } else { Vec::new() };
    let acps = app.agents_settings.entries.iter().filter(|entry| entry.can_start_session()).map(|entry| SlashPaletteCandidate {
        id: entry.key.clone(), label: entry.label.clone(), description: Some(entry.description.clone()),
    }).collect();
    desktop_chat_input::input_panel::suggestions::candidates(
        desktop_chat_input::input_panel::suggestions::CandidateInput {
            skills, tools, acps, providers: &app.model_settings.providers,
            mcp_editor: &app.mcp_settings.editor.text(), started: app.current_session_has_started(), acp: runtime.acp_agent.is_some(),
        },
    )
}

pub(crate) fn ranked_slash_palette_sections(app: &App) -> Vec<SlashPaletteSection> {
    build_ranked_slash_sections(
        &app.input_suggestions.slash_palette_query,
        &collect_slash_palette_candidates(app),
        &app.input_suggestions.slash_palette_expanded_kinds,
    )
}

fn visible_slash_entries(app: &App) -> Vec<SlashPaletteEntry> {
    flatten_visible_slash_entries(&ranked_slash_palette_sections(app))
}

pub(super) fn scroll_slash_palette_to_top(app: &App) -> Task<Message> {
    iced::widget::operation::snap_to(
        app.slash_palette_scroll_id.clone(),
        iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(0.0) },
    )
    .map(|_: ()| Message::None)
}

fn scroll_slash_palette_to_selected(app: &App) -> Task<Message> {
    let sections = ranked_slash_palette_sections(app);
    let (selected_row, total_rows) =
        slash_palette_scroll_row(&sections, app.input_suggestions.slash_palette_selected_index);
    let y = slash_palette_scroll_relative_y(selected_row, total_rows);
    iced::widget::operation::snap_to(
        app.slash_palette_scroll_id.clone(),
        iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(y) },
    )
    .map(|_: ()| Message::None)
}

#[cfg(test)]
pub(crate) fn open_slash_palette(app: &mut App, query: &str) -> Task<Message> {
    let effects = app.input_suggestions.open_slash(query, app.skills_settings.catalog.is_empty(), app.skills_settings.loading);
    super::file_search::apply_suggestion_effects(app, effects)
}

pub(super) fn handle_slash_palette_closed(app: &mut App) -> Task<Message> {
    clear_slash_palette_state(app);
    focus_input_editor(app)
}

pub(super) fn handle_slash_palette_navigate_up(app: &mut App) -> Task<Message> {
    let total = visible_slash_entries(app).len();
    app.input_suggestions.navigate_slash(false, total);
    scroll_slash_palette_to_selected(app)
}

pub(super) fn handle_slash_palette_navigate_down(app: &mut App) -> Task<Message> {
    let total = visible_slash_entries(app).len();
    app.input_suggestions.navigate_slash(true, total);
    scroll_slash_palette_to_selected(app)
}

pub(super) fn handle_slash_palette_expand_kind(
    app: &mut App,
    kind: SlashPaletteKind,
) -> Task<Message> {
    app.input_suggestions.slash_palette_expanded_kinds.insert(kind);
    Task::none()
}

pub(super) fn handle_slash_palette_select_current(app: &mut App) -> Task<Message> {
    let entries = visible_slash_entries(app);
    let Some(entry) = entries.get(app.input_suggestions.slash_palette_selected_index).cloned() else {
        return Task::none();
    };
    apply_slash_palette_entry(app, entry)
}

pub(super) fn handle_slash_palette_select(
    app: &mut App,
    kind: SlashPaletteKind,
    id: String,
) -> Task<Message> {
    let entries = visible_slash_entries(app);
    let entry = entries
        .into_iter()
        .find(|entry| entry.kind == kind && entry.id == id)
        .unwrap_or(SlashPaletteEntry { kind, id, label: String::new(), description: None });
    apply_slash_palette_entry(app, entry)
}

fn clear_slash_token_from_input(app: &mut App) {
    let mut state = std::mem::take(&mut app.input_suggestions);
    state.clear_slash_token(&mut app.current_session_runtime_mut().input_editor);
    app.input_suggestions = state;
    sync_global_input_editor_if_needed(app);
    app.focus_area = FocusArea::None;
}

fn apply_slash_palette_entry(app: &mut App, entry: SlashPaletteEntry) -> Task<Message> {
    clear_slash_token_from_input(app);
    let focus = focus_input_editor(app);
    use desktop_chat_input::input_panel::suggestions::{selection_effect, SelectionEffect};
    let apply = match selection_effect(entry) {
        SelectionEffect::Model(id) => crate::app::message::chat::session::update(
            app,
            message::ChatMessage::ModelSelected(id),
        ),
        SelectionEffect::Agent(agent) => {
            crate::app::message::chat::session::update(
                app,
                message::ChatMessage::SessionAgentSelected(agent),
            )
        }
        SelectionEffect::Tool(id) => crate::app::message::chat::session::update(
            app,
            message::ChatMessage::SessionManualToolSelected(id),
        ),
        SelectionEffect::Skill(id) => crate::app::message::chat::session::update(
            app,
            message::ChatMessage::SessionSkillSelected(id),
        ),
        SelectionEffect::Mcp(id) => apply_mcp_toggle(app, &id),
    };
    Task::batch(vec![focus, apply])
}

fn apply_mcp_toggle(app: &mut App, server_name: &str) -> Task<Message> {
    let Some((index, enabled)) = desktop_chat_input::input_panel::suggestions::mcp_toggle(&app.mcp_settings.editor.text(), server_name)
    else {
        app.push_localized_notification(
            "未找到 MCP 服务器：%s",
            vec![format!("{}", server_name).into()],
        );
        return Task::none();
    };
    Task::done(Message::Settings(message::SettingsMessage::Mcp(
        message::settings::McpMessage::ServerEnabledToggled(index, enabled),
    )))
}

pub(crate) use desktop_session::message::chat::input::slash_palette::*;
