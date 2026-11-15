//! Slash palette 查询检测与分组候选排序。
//!
//! 本模块负责识别 chat 输入尾部 `/query`，并按 Skills / Tools / MCP / Models / ACP
//! 分组构建可折叠的模糊排序结果，供状态机与 overlay UI 复用。

use super::shared::{focus_input_editor, sync_global_input_editor_if_needed};
use crate::app::state::tool_display_name;
use crate::app::{App, FocusArea, Message, message};
use iced::Task;
use iced::widget::text_editor;

pub(crate) fn clear_slash_palette_state(app: &mut App) {
    app.show_slash_palette = false;
    app.slash_palette_query.clear();
    app.slash_palette_selected_index = 0;
    app.slash_palette_expanded_kinds.clear();
}

pub(crate) fn collect_slash_palette_candidates(app: &App) -> SlashPaletteCandidates {
    let skills = app
        .skills_settings
        .catalog
        .iter()
        .map(|skill| SlashPaletteCandidate {
            id: skill.id.clone(),
            label: if skill.title.trim().is_empty() {
                skill.id.clone()
            } else {
                skill.title.clone()
            },
            description: {
                let desc = skill.description.trim();
                (!desc.is_empty()).then(|| desc.to_string())
            },
        })
        .collect();

    let runtime = app.current_session_runtime();
    let tools = if runtime.acp_agent.is_none() {
        app.session_tool_inventory(&runtime)
            .base_tools
            .into_iter()
            .filter(|tool_id| !tool_id.trim().is_empty())
            .map(|tool_id| {
                let label = tool_display_name(&tool_id);
                SlashPaletteCandidate { id: tool_id, label, description: None }
            })
            .collect()
    } else {
        Vec::new()
    };

    let mcps = mcp_candidates_from_editor(&app.mcp_settings.editor.text())
        .into_iter()
        .map(|(_, _, candidate)| candidate)
        .collect();

    if app.current_session_has_started() {
        return SlashPaletteCandidates { skills, tools, mcps, ..Default::default() };
    }

    let mut models = vec![SlashPaletteCandidate {
        id: "auto".to_string(),
        label: "Auto".to_string(),
        description: Some("自动选择模型".to_string()),
    }];
    for provider in &app.model_settings.providers {
        for model in &provider.models {
            if !model.enabled {
                continue;
            }
            let id = format!("{}/{}", provider.id, model.id);
            models.push(SlashPaletteCandidate {
                id,
                label: model.name.clone(),
                description: Some(provider.name.clone()),
            });
        }
    }

    let acps = app
        .agents_settings
        .entries
        .iter()
        .filter(|entry| entry.can_start_session())
        .map(|entry| SlashPaletteCandidate {
            id: entry.key.clone(),
            label: entry.label.clone(),
            description: Some(entry.description.clone()),
        })
        .collect();

    SlashPaletteCandidates { skills, tools, mcps, models, acps }
}

pub(crate) fn ranked_slash_palette_sections(app: &App) -> Vec<SlashPaletteSection> {
    build_ranked_slash_sections(
        &app.slash_palette_query,
        &collect_slash_palette_candidates(app),
        &app.slash_palette_expanded_kinds,
    )
}

fn visible_slash_entries(app: &App) -> Vec<SlashPaletteEntry> {
    flatten_visible_slash_entries(&ranked_slash_palette_sections(app))
}

fn scroll_slash_palette_to_top(app: &App) -> Task<Message> {
    iced::widget::operation::snap_to(
        app.slash_palette_scroll_id.clone(),
        iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(0.0) },
    )
    .map(|_: ()| Message::None)
}

fn scroll_slash_palette_to_selected(app: &App) -> Task<Message> {
    let sections = ranked_slash_palette_sections(app);
    let (selected_row, total_rows) =
        slash_palette_scroll_row(&sections, app.slash_palette_selected_index);
    let y = slash_palette_scroll_relative_y(selected_row, total_rows);
    iced::widget::operation::snap_to(
        app.slash_palette_scroll_id.clone(),
        iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(y) },
    )
    .map(|_: ()| Message::None)
}

pub(crate) fn open_slash_palette(app: &mut App, query: &str) -> Task<Message> {
    let query_changed = app.slash_palette_query != query;
    app.show_slash_palette = true;
    app.slash_palette_query = query.to_string();
    app.slash_palette_selected_index = 0;
    clear_file_search_only(app);

    let focus_task = focus_input_editor(app);
    let refresh_skills = if app.skills_settings.catalog.is_empty() && !app.skills_settings.loading {
        Task::done(Message::Settings(message::SettingsMessage::SkillsRefresh))
    } else {
        Task::none()
    };
    if query_changed {
        Task::batch(vec![focus_task, scroll_slash_palette_to_top(app), refresh_skills])
    } else {
        Task::batch(vec![focus_task, refresh_skills])
    }
}

fn clear_file_search_only(app: &mut App) {
    app.show_file_search = false;
    app.file_search_query.clear();
    app.file_search_selected_index = 0;
}

pub(super) fn handle_slash_palette_closed(app: &mut App) -> Task<Message> {
    clear_slash_palette_state(app);
    focus_input_editor(app)
}

pub(super) fn handle_slash_palette_navigate_up(app: &mut App) -> Task<Message> {
    if app.slash_palette_selected_index > 0 {
        app.slash_palette_selected_index -= 1;
    }
    scroll_slash_palette_to_selected(app)
}

pub(super) fn handle_slash_palette_navigate_down(app: &mut App) -> Task<Message> {
    let entries = visible_slash_entries(app);
    if app.slash_palette_selected_index < entries.len().saturating_sub(1) {
        app.slash_palette_selected_index += 1;
    }
    scroll_slash_palette_to_selected(app)
}

pub(super) fn handle_slash_palette_expand_kind(
    app: &mut App,
    kind: SlashPaletteKind,
) -> Task<Message> {
    app.slash_palette_expanded_kinds.insert(kind);
    Task::none()
}

pub(super) fn handle_slash_palette_select_current(app: &mut App) -> Task<Message> {
    let entries = visible_slash_entries(app);
    let Some(entry) = entries.get(app.slash_palette_selected_index).cloned() else {
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
    let text = {
        let runtime = app.current_session_runtime();
        clear_trailing_slash_token(&runtime.input_editor.text())
    };
    let runtime = app.current_session_runtime_mut();
    runtime.input_editor = text_editor::Content::with_text(&text);
    runtime.input_editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
    sync_global_input_editor_if_needed(app);
    clear_slash_palette_state(app);
    app.focus_area = FocusArea::None;
}

fn apply_slash_palette_entry(app: &mut App, entry: SlashPaletteEntry) -> Task<Message> {
    clear_slash_token_from_input(app);
    let focus = focus_input_editor(app);
    let apply = match entry.kind {
        SlashPaletteKind::Model => crate::app::message::chat::session::update(
            app,
            message::ChatMessage::ModelSelected(entry.id),
        ),
        SlashPaletteKind::Acp => {
            let agent = if entry.id.trim().is_empty() { None } else { Some(entry.id) };
            crate::app::message::chat::session::update(
                app,
                message::ChatMessage::SessionAgentSelected(agent),
            )
        }
        SlashPaletteKind::Tool => crate::app::message::chat::session::update(
            app,
            message::ChatMessage::SessionManualToolSelected(entry.id),
        ),
        SlashPaletteKind::Skill => crate::app::message::chat::session::update(
            app,
            message::ChatMessage::SessionSkillSelected(entry.id),
        ),
        SlashPaletteKind::Mcp => apply_mcp_toggle(app, &entry.id),
    };
    Task::batch(vec![focus, apply])
}

fn apply_mcp_toggle(app: &mut App, server_name: &str) -> Task<Message> {
    let servers = mcp_candidates_from_editor(&app.mcp_settings.editor.text());
    let Some((index, enabled, _)) =
        servers.into_iter().find(|(_, _, candidate)| candidate.id == server_name)
    else {
        app.push_localized_notification(
            "未找到 MCP 服务器：%s",
            vec![format!("{}", server_name).into()],
        );
        return Task::none();
    };
    Task::done(Message::Settings(message::SettingsMessage::Mcp(
        message::settings::McpMessage::ServerEnabledToggled(index, !enabled),
    )))
}

pub(crate) use desktop_session::message::chat::input::slash_palette::*;
