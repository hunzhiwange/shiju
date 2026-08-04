//! Slash palette 查询检测与分组候选排序。
//!
//! 本模块负责识别 chat 输入尾部 `/query`，并按 Skills / Tools / MCP / Models / ACP
//! 分组构建可折叠的模糊排序结果，供状态机与 overlay UI 复用。

use super::file_search::{fuzzy_subsequence_score, trailing_token};
use super::shared::{focus_input_editor, sync_global_input_editor_if_needed};
use crate::app::state::{SlashPaletteKind, tool_display_name};
use crate::app::{App, FocusArea, Message, message};
use iced::Task;
use iced::widget::text_editor;
use serde_json::Value;
use std::collections::BTreeSet;

/// 空查询时每组默认可见条目数。
pub(crate) const SLASH_PALETTE_DEFAULT_VISIBLE: usize = 3;
/// 每组匹配结果上限。
pub(crate) const SLASH_PALETTE_MAX_PER_KIND: usize = 50;

/// 单个 slash 候选项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlashPaletteEntry {
    pub(crate) kind: SlashPaletteKind,
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) description: Option<String>,
}

/// 构建候选时使用的原始输入项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlashPaletteCandidate {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) description: Option<String>,
}

/// 一个类别分区的可见结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlashPaletteSection {
    pub(crate) kind: SlashPaletteKind,
    pub(crate) entries: Vec<SlashPaletteEntry>,
    pub(crate) hidden_count: usize,
}

/// 构建分组结果时的全部候选池。
#[derive(Debug, Clone, Default)]
pub(crate) struct SlashPaletteCandidates {
    pub(crate) skills: Vec<SlashPaletteCandidate>,
    pub(crate) tools: Vec<SlashPaletteCandidate>,
    pub(crate) mcps: Vec<SlashPaletteCandidate>,
    pub(crate) models: Vec<SlashPaletteCandidate>,
    pub(crate) acps: Vec<SlashPaletteCandidate>,
}

pub(crate) fn is_slash_query_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '/' | '.' | ':')
}

/// 若尾部 token 以 `/` 开头且后续字符合法，返回 query（不含前导 `/`）。
pub(crate) fn active_slash_query(text: &str) -> Option<&str> {
    let token = trailing_token(text);
    let query = token.strip_prefix('/')?;
    query.chars().all(is_slash_query_char).then_some(query)
}

/// 清除尾部活跃的 `/query` token；若无活跃 slash，原样返回。
pub(crate) fn clear_trailing_slash_token(text: &str) -> String {
    let Some(_) = active_slash_query(text) else {
        return text.to_string();
    };
    let token = trailing_token(text);
    let token_start = text.len().saturating_sub(token.len());
    let mut new_text = text[..token_start].to_string();
    while new_text.ends_with(|c: char| c.is_whitespace()) {
        new_text.pop();
    }
    new_text
}

fn match_score(candidate: &SlashPaletteCandidate, query: &str) -> Option<i64> {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return Some(0);
    }
    let id = candidate.id.to_ascii_lowercase();
    let label = candidate.label.to_ascii_lowercase();
    let desc = candidate.description.as_deref().map(str::to_ascii_lowercase).unwrap_or_default();

    let id_score = fuzzy_subsequence_score(&id, &query).unwrap_or(i64::MIN / 2);
    let label_score = fuzzy_subsequence_score(&label, &query).unwrap_or(i64::MIN / 2);
    let desc_score = if desc.is_empty() {
        i64::MIN / 2
    } else {
        fuzzy_subsequence_score(&desc, &query).unwrap_or(i64::MIN / 2).saturating_sub(40)
    };

    let mut score = id_score.max(label_score).max(desc_score);
    if score == i64::MIN / 2 {
        return None;
    }
    if id == query || label == query {
        score += 2000;
    }
    if id.starts_with(&query) || label.starts_with(&query) {
        score += 800;
    }
    Some(score)
}

fn rank_kind(
    kind: SlashPaletteKind,
    candidates: &[SlashPaletteCandidate],
    query: &str,
    expanded: bool,
    force_expand_for_query: bool,
) -> Option<SlashPaletteSection> {
    let mut ranked = candidates
        .iter()
        .filter_map(|candidate| {
            let score = match_score(candidate, query)?;
            Some((
                score,
                SlashPaletteEntry {
                    kind,
                    id: candidate.id.clone(),
                    label: candidate.label.clone(),
                    description: candidate.description.clone(),
                },
            ))
        })
        .collect::<Vec<_>>();

    if ranked.is_empty() {
        return None;
    }

    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.label.cmp(&b.1.label)));
    ranked.truncate(SLASH_PALETTE_MAX_PER_KIND);

    let entries: Vec<SlashPaletteEntry> = ranked.into_iter().map(|(_, entry)| entry).collect();
    let show_all = expanded || force_expand_for_query || !query.trim().is_empty();
    if show_all || entries.len() <= SLASH_PALETTE_DEFAULT_VISIBLE {
        return Some(SlashPaletteSection { kind, entries, hidden_count: 0 });
    }

    let hidden_count = entries.len().saturating_sub(SLASH_PALETTE_DEFAULT_VISIBLE);
    let visible = entries.into_iter().take(SLASH_PALETTE_DEFAULT_VISIBLE).collect();
    Some(SlashPaletteSection { kind, entries: visible, hidden_count })
}

/// 按固定类别顺序构建分组结果。
pub(crate) fn build_ranked_slash_sections(
    query: &str,
    candidates: &SlashPaletteCandidates,
    expanded_kinds: &BTreeSet<SlashPaletteKind>,
) -> Vec<SlashPaletteSection> {
    let force_expand_for_query = !query.trim().is_empty();
    SlashPaletteKind::all()
        .into_iter()
        .filter_map(|kind| {
            let pool = match kind {
                SlashPaletteKind::Skill => &candidates.skills,
                SlashPaletteKind::Tool => &candidates.tools,
                SlashPaletteKind::Mcp => &candidates.mcps,
                SlashPaletteKind::Model => &candidates.models,
                SlashPaletteKind::Acp => &candidates.acps,
            };
            rank_kind(kind, pool, query, expanded_kinds.contains(&kind), force_expand_for_query)
        })
        .collect()
}

/// 将当前可见分组条目展平，供键盘选中索引使用。
pub(crate) fn flatten_visible_slash_entries(
    sections: &[SlashPaletteSection],
) -> Vec<SlashPaletteEntry> {
    sections.iter().flat_map(|section| section.entries.iter().cloned()).collect()
}

pub(crate) fn clear_slash_palette_state(app: &mut App) {
    app.show_slash_palette = false;
    app.slash_palette_query.clear();
    app.slash_palette_selected_index = 0;
    app.slash_palette_expanded_kinds.clear();
}

fn mcp_candidates_from_editor(editor_text: &str) -> Vec<(usize, bool, SlashPaletteCandidate)> {
    let Ok(value) = serde_json::from_str::<Value>(editor_text) else {
        return Vec::new();
    };
    let Some(servers) = value.as_array() else {
        return Vec::new();
    };
    servers
        .iter()
        .enumerate()
        .filter_map(|(index, server)| {
            let name = server.get("name")?.as_str()?.trim();
            if name.is_empty() {
                return None;
            }
            let enabled = server.get("enabled").and_then(Value::as_bool).unwrap_or(true);
            Some((
                index,
                enabled,
                SlashPaletteCandidate {
                    id: name.to_string(),
                    label: name.to_string(),
                    description: Some(if enabled {
                        "已启用".to_string()
                    } else {
                        "已禁用".to_string()
                    }),
                },
            ))
        })
        .collect()
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
    let tools = app
        .session_tool_inventory(&runtime)
        .base_tools
        .into_iter()
        .filter(|tool_id| !tool_id.trim().is_empty())
        .map(|tool_id| {
            let label = tool_display_name(&tool_id);
            SlashPaletteCandidate { id: tool_id, label, description: None }
        })
        .collect();

    let mcps = mcp_candidates_from_editor(&app.mcp_settings.editor.text())
        .into_iter()
        .map(|(_, _, candidate)| candidate)
        .collect();

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

    let mut acps = vec![SlashPaletteCandidate {
        id: String::new(),
        label: "默认智能体".to_string(),
        description: Some("清除 ACP 选择".to_string()),
    }];
    for agent in &app.acp_agents {
        let trimmed = agent.trim();
        if trimmed.is_empty() {
            continue;
        }
        acps.push(SlashPaletteCandidate {
            id: trimmed.to_string(),
            label: trimmed.to_string(),
            description: None,
        });
    }

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

/// 计算选中条目在滚动列表中的可视行位置。
///
/// 返回 `(selected_row, total_rows)`，`total_rows` 计入分组标题、条目、Show more 与分隔行，
/// 以便相对滚动偏移与实际滚动内容对齐。
pub(super) fn slash_palette_scroll_row(
    sections: &[SlashPaletteSection],
    selected_index: usize,
) -> (usize, usize) {
    let mut row = 0usize;
    let mut selected_row = 0usize;
    let mut flat_index = 0usize;

    for section in sections {
        // section header
        row += 1;
        for _entry in &section.entries {
            if flat_index == selected_index {
                selected_row = row;
            }
            row += 1;
            flat_index += 1;
        }
        if section.hidden_count > 0 {
            row += 1;
        }
        // trailing spacer
        row += 1;
    }

    if flat_index == 0 {
        return (0, row.max(1));
    }
    (selected_row, row.max(1))
}

fn scroll_slash_palette_to_top(app: &App) -> Task<Message> {
    iced::widget::operation::snap_to(
        app.slash_palette_scroll_id.clone(),
        iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(0.0) },
    )
    .map(|_: ()| Message::None)
}

/// 计算让选中行落在列表中的相对滚动位置（0=顶，1=底）。
pub(super) fn slash_palette_scroll_relative_y(selected_row: usize, total_rows: usize) -> f32 {
    if total_rows <= 1 {
        return 0.0;
    }
    (selected_row.min(total_rows.saturating_sub(1)) as f32) / ((total_rows - 1) as f32)
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
                message::ChatMessage::AcpAgentSelected(agent),
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
        app.push_notification(format!("未找到 MCP 服务器：{server_name}"));
        return Task::none();
    };
    Task::done(Message::Settings(message::SettingsMessage::Mcp(
        message::settings::McpMessage::ServerEnabledToggled(index, !enabled),
    )))
}
