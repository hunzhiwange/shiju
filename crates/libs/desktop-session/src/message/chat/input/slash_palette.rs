use super::file_search::{fuzzy_subsequence_score, trailing_token};
use std::collections::BTreeSet;
use serde_json::Value;
/// Slash 面板条目类别，顺序固定为 Skills → Tools → MCP → Models → ACP。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SlashPaletteKind {
    Skill,
    Tool,
    Mcp,
    Model,
    Acp,
}

impl SlashPaletteKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Skill => "Skills",
            Self::Tool => "Tools",
            Self::Mcp => "MCP",
            Self::Model => "Models",
            Self::Acp => "智能体",
        }
    }

    pub fn all() -> [Self; 5] {
        [Self::Skill, Self::Tool, Self::Mcp, Self::Model, Self::Acp]
    }
}

/// 空查询时每组默认可见条目数。
pub const SLASH_PALETTE_DEFAULT_VISIBLE: usize = 3;
/// 每组匹配结果上限。
pub const SLASH_PALETTE_MAX_PER_KIND: usize = 50;

/// 单个 slash 候选项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashPaletteEntry {
    pub kind: SlashPaletteKind,
    pub id: String,
    pub label: String,
    pub description: Option<String>,
}

/// 构建候选时使用的原始输入项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashPaletteCandidate {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
}

/// 一个类别分区的可见结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashPaletteSection {
    pub kind: SlashPaletteKind,
    pub entries: Vec<SlashPaletteEntry>,
    pub hidden_count: usize,
}

/// 构建分组结果时的全部候选池。
#[derive(Debug, Clone, Default)]
pub struct SlashPaletteCandidates {
    pub skills: Vec<SlashPaletteCandidate>,
    pub tools: Vec<SlashPaletteCandidate>,
    pub mcps: Vec<SlashPaletteCandidate>,
    pub models: Vec<SlashPaletteCandidate>,
    pub acps: Vec<SlashPaletteCandidate>,
}

pub fn is_slash_query_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '/' | '.' | ':')
}

pub fn active_slash_query(text: &str) -> Option<&str> {
    let token = trailing_token(text);
    let query = token.strip_prefix('/')?;
    query.chars().all(is_slash_query_char).then_some(query)
}

pub fn clear_trailing_slash_token(text: &str) -> String {
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
    let mut label = candidate.label.to_ascii_lowercase();
    let built_in = match candidate.id.as_str() {
        "auto" if candidate.label == "Auto" => Some(("自动模型", "自动选择模型")),
        "" if candidate.label == "默认智能体" => Some(("默认智能体", "清除 ACP 选择")),
        _ => None,
    };
    if let Some((title, description)) = built_in {
        for locale in [
            config_ui::DesktopLocale::SimplifiedChinese,
            config_ui::DesktopLocale::TraditionalChinese,
            config_ui::DesktopLocale::English,
        ] {
            let i18n = desktop_i18n::I18n::new(locale);
            label.push(' ');
            label.push_str(
                &format!("{} {}", i18n.__(title), i18n.__(description)).to_ascii_lowercase(),
            );
        }
    }
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

pub fn build_ranked_slash_sections(
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

pub fn flatten_visible_slash_entries(
    sections: &[SlashPaletteSection],
) -> Vec<SlashPaletteEntry> {
    sections.iter().flat_map(|section| section.entries.iter().cloned()).collect()
}

pub fn mcp_candidates_from_editor(editor_text: &str) -> Vec<(usize, bool, SlashPaletteCandidate)> {
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

pub fn slash_palette_scroll_row(
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

pub fn slash_palette_scroll_relative_y(selected_row: usize, total_rows: usize) -> f32 {
    if total_rows <= 1 {
        return 0.0;
    }
    (selected_row.min(total_rows.saturating_sub(1)) as f32) / ((total_rows - 1) as f32)
}