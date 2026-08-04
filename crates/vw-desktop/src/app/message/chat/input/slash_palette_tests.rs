//! Slash palette 查询与分组排序测试。

use super::slash_palette::{
    SlashPaletteCandidate, SlashPaletteCandidates, active_slash_query, build_ranked_slash_sections,
    clear_trailing_slash_token, slash_palette_scroll_relative_y, slash_palette_scroll_row,
};
use crate::app::state::SlashPaletteKind;
use std::collections::BTreeSet;

fn candidate(id: &str, label: &str) -> SlashPaletteCandidate {
    SlashPaletteCandidate { id: id.to_string(), label: label.to_string(), description: None }
}

fn sample_candidates() -> SlashPaletteCandidates {
    SlashPaletteCandidates {
        skills: vec![
            candidate("automate", "automate"),
            candidate("babysit", "babysit"),
            candidate("baidu-trending", "baidu-trending"),
            candidate("code-review", "code-review"),
            candidate("writing-plans", "writing-plans"),
        ],
        tools: vec![
            candidate("read", "read"),
            candidate("write", "write"),
            candidate("shell", "shell"),
            candidate("web_search", "web_search"),
        ],
        mcps: vec![
            candidate("filesystem", "filesystem"),
            candidate("github", "github"),
            candidate("linear", "linear"),
        ],
        models: vec![
            candidate("auto", "Auto"),
            candidate("openai/gpt-4.1", "gpt-4.1"),
            candidate("anthropic/claude-sonnet", "claude-sonnet"),
        ],
        acps: vec![
            candidate("codex", "Codex"),
            candidate("claude-code", "Claude Code"),
            candidate("opencode", "OpenCode"),
        ],
    }
}

#[test]
fn active_slash_query_accepts_prefix_and_empty_query() {
    assert_eq!(active_slash_query("/"), Some(""));
    assert_eq!(active_slash_query("hi /gpt"), Some("gpt"));
    assert_eq!(active_slash_query("prefix /claude-sonnet"), Some("claude-sonnet"));
    assert_eq!(active_slash_query("/openai/gpt-4.1"), Some("openai/gpt-4.1"));
}

#[test]
fn active_slash_query_rejects_non_slash_trailing_tokens() {
    assert_eq!(active_slash_query("plain text"), None);
    assert_eq!(active_slash_query("@src"), None);
    assert_eq!(active_slash_query("open @src/main.rs"), None);
    assert_eq!(active_slash_query("path/to/file"), None);
    assert_eq!(active_slash_query("/bad query!"), None);
}

#[test]
fn clear_trailing_slash_token_removes_active_slash_query() {
    assert_eq!(clear_trailing_slash_token("hello /gpt"), "hello");
    assert_eq!(clear_trailing_slash_token("/"), "");
    assert_eq!(clear_trailing_slash_token("keep @file"), "keep @file");
}

#[test]
fn build_ranked_slash_sections_groups_filters_and_collapses_by_default() {
    let candidates = sample_candidates();
    let sections = build_ranked_slash_sections("", &candidates, &BTreeSet::new());

    assert_eq!(
        sections.iter().map(|section| section.kind).collect::<Vec<_>>(),
        vec![
            SlashPaletteKind::Skill,
            SlashPaletteKind::Tool,
            SlashPaletteKind::Mcp,
            SlashPaletteKind::Model,
            SlashPaletteKind::Acp,
        ]
    );

    let skills = sections.iter().find(|section| section.kind == SlashPaletteKind::Skill).unwrap();
    assert_eq!(skills.entries.len(), 3);
    assert_eq!(skills.hidden_count, 2);

    let tools = sections.iter().find(|section| section.kind == SlashPaletteKind::Tool).unwrap();
    assert_eq!(tools.entries.len(), 3);
    assert_eq!(tools.hidden_count, 1);

    let filtered = build_ranked_slash_sections("gpt", &candidates, &BTreeSet::new());
    assert!(filtered.iter().all(|section| section.hidden_count == 0));
    assert!(filtered.iter().any(|section| section.kind == SlashPaletteKind::Model
        && section.entries.iter().any(|entry| entry.id.contains("gpt"))));
    assert!(!filtered.iter().any(|section| section.kind == SlashPaletteKind::Skill));
}

#[test]
fn build_ranked_slash_sections_expands_requested_kind() {
    let candidates = sample_candidates();
    let mut expanded = BTreeSet::new();
    expanded.insert(SlashPaletteKind::Skill);

    let sections = build_ranked_slash_sections("", &candidates, &expanded);
    let skills = sections.iter().find(|section| section.kind == SlashPaletteKind::Skill).unwrap();
    assert_eq!(skills.entries.len(), 5);
    assert_eq!(skills.hidden_count, 0);

    let tools = sections.iter().find(|section| section.kind == SlashPaletteKind::Tool).unwrap();
    assert_eq!(tools.entries.len(), 3);
    assert_eq!(tools.hidden_count, 1);
}

#[test]
fn slash_palette_scroll_row_accounts_for_headers_and_show_more() {
    let candidates = sample_candidates();
    let sections = build_ranked_slash_sections("", &candidates, &BTreeSet::new());

    // First entry sits under the Skills header => visual row 1.
    let (first_row, total_rows) = slash_palette_scroll_row(&sections, 0);
    assert_eq!(first_row, 1);
    assert!(total_rows > first_row);

    // Moving deeper should increase the visual row.
    let (later_row, _) = slash_palette_scroll_row(&sections, 5);
    assert!(later_row > first_row);

    // Last entry should not clamp past the last visual row.
    let entry_count: usize = sections.iter().map(|section| section.entries.len()).sum();
    let (last_row, total_rows) = slash_palette_scroll_row(&sections, entry_count.saturating_sub(1));
    assert!(last_row < total_rows);
}

#[test]
fn slash_palette_scroll_relative_y_maps_row_to_unit_interval() {
    assert_eq!(slash_palette_scroll_relative_y(0, 0), 0.0);
    assert_eq!(slash_palette_scroll_relative_y(0, 1), 0.0);
    assert_eq!(slash_palette_scroll_relative_y(0, 5), 0.0);
    assert!((slash_palette_scroll_relative_y(2, 5) - 0.5).abs() < f32::EPSILON);
    assert_eq!(slash_palette_scroll_relative_y(4, 5), 1.0);
    assert_eq!(slash_palette_scroll_relative_y(99, 5), 1.0);
}
