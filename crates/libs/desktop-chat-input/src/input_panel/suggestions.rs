//! 输入建议状态及编辑转换。系统焦点、索引刷新和滚动由宿主承接。
use desktop_session::message::chat::input::{
    file_search::active_file_search_query,
    slash_palette::{SlashPaletteKind, active_slash_query},
};
use iced::widget::text_editor;
use std::collections::BTreeSet;

#[derive(Debug, Default)]
pub struct State {
    pub file_search_cache_query: String,
    pub file_search_cache_project_path: Option<String>,
    pub file_search_cache_revision: u64,
    pub file_search_cache_entries:
        Vec<desktop_session::message::chat::input::file_search::FileSearchResult>,
    pub file_ref_hovered_index: Option<usize>,
    pub file_search_query: String,
    pub show_file_search: bool,
    pub file_search_selected_index: usize,
    pub slash_palette_query: String,
    pub show_slash_palette: bool,
    pub slash_palette_selected_index: usize,
    pub slash_palette_expanded_kinds: BTreeSet<SlashPaletteKind>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Effects {
    pub focus: bool,
    pub refresh_files: bool,
    pub scroll_files_to_top: bool,
    pub scroll_slash_to_top: bool,
    pub refresh_skills: bool,
}
impl State {
    pub fn clear_files(&mut self) {
        self.show_file_search = false;
        self.file_search_query.clear();
        self.file_search_selected_index = 0;
    }
    pub fn clear_slash(&mut self) {
        self.show_slash_palette = false;
        self.slash_palette_query.clear();
        self.slash_palette_selected_index = 0;
        self.slash_palette_expanded_kinds.clear();
    }
    pub fn open_slash(&mut self, query: &str, skills_empty: bool, skills_loading: bool) -> Effects {
        let changed = self.slash_palette_query != query;
        self.show_slash_palette = true;
        self.slash_palette_query = query.to_string();
        self.slash_palette_selected_index = 0;
        self.clear_files();
        Effects {
            focus: true,
            scroll_slash_to_top: changed,
            refresh_skills: skills_empty && !skills_loading,
            ..Default::default()
        }
    }
    pub fn change_files(&mut self, value: String) -> Effects {
        let changed = self.file_search_query != value;
        self.show_file_search = !value.trim().is_empty();
        self.file_search_query = value;
        self.file_search_selected_index = 0;
        Effects {
            focus: true,
            refresh_files: true,
            scroll_files_to_top: self.show_file_search && changed,
            ..Default::default()
        }
    }
    pub fn editor_action(
        &mut self,
        editor: &mut text_editor::Content,
        action: text_editor::Action,
        skills_empty: bool,
        skills_loading: bool,
    ) -> Effects {
        let edited = matches!(&action, text_editor::Action::Edit(_));
        editor.perform(action);
        if !edited {
            return Effects::default();
        }
        let text = editor.text();
        let old_file = self.file_search_query.clone();
        let old_slash = self.slash_palette_query.clone();
        if !text.chars().last().is_some_and(char::is_whitespace) {
            if let Some(query) = active_slash_query(&text) {
                return self.open_slash(query, skills_empty, skills_loading);
            }
            if let Some(query) = active_file_search_query(&text) {
                self.clear_slash();
                self.show_file_search = true;
                self.file_search_query = query.to_string();
                self.file_search_selected_index = 0;
                return Effects {
                    focus: true,
                    refresh_files: true,
                    scroll_files_to_top: old_file != self.file_search_query
                        || old_slash != self.slash_palette_query,
                    ..Default::default()
                };
            }
        }
        self.clear_files();
        self.clear_slash();
        Effects { refresh_files: true, ..Default::default() }
    }
    pub fn navigate_file(&mut self, down: bool, total: usize) {
        if down {
            if self.file_search_selected_index < total.min(20).saturating_sub(1) {
                self.file_search_selected_index += 1;
            }
        } else {
            self.file_search_selected_index = self.file_search_selected_index.saturating_sub(1);
        }
    }
    pub fn navigate_slash(&mut self, down: bool, total: usize) {
        if down {
            if self.slash_palette_selected_index < total.saturating_sub(1) {
                self.slash_palette_selected_index += 1;
            }
        } else {
            self.slash_palette_selected_index = self.slash_palette_selected_index.saturating_sub(1);
        }
    }
    pub fn clear_slash_token(&mut self, editor: &mut text_editor::Content) {
        let text = desktop_session::message::chat::input::slash_palette::clear_trailing_slash_token(
            &editor.text(),
        );
        *editor = text_editor::Content::with_text(&text);
        editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
        self.clear_slash();
    }
}

/// 插入选中的路径并将光标移动到文尾。
pub fn select_file(editor: &mut text_editor::Content, project_root: Option<&str>, path: &str) {
    use desktop_editor::components::input_mention_highlighter::file_mention;
    use desktop_session::message::chat::input::file_search::replace_or_append_file_mention;
    let rel_or_full_path = if let Some(project_root) = project_root {
        std::path::Path::new(path)
            .strip_prefix(project_root)
            .ok()
            .and_then(|entry| entry.to_str())
            .unwrap_or(path)
            .replace('\\', "/")
    } else {
        path.replace('\\', "/")
    };
    let replacement = file_mention(&rel_or_full_path);

    let text = replace_or_append_file_mention(&editor.text(), &replacement);
    *editor = text_editor::Content::with_text(&text);
    editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
}

impl State {
    pub fn select_file(
        &mut self,
        editor: &mut text_editor::Content,
        project_root: Option<&str>,
        path: &str,
    ) -> Effects {
        select_file(editor, project_root, path);
        self.clear_files();
        Effects { focus: true, refresh_files: true, ..Default::default() }
    }
}

pub struct CandidateInput<'a> {
    pub skills: Vec<desktop_session::message::chat::input::slash_palette::SlashPaletteCandidate>,
    pub tools: Vec<desktop_session::message::chat::input::slash_palette::SlashPaletteCandidate>,
    pub acps: Vec<desktop_session::message::chat::input::slash_palette::SlashPaletteCandidate>,
    pub providers: &'a [desktop_settings_providers::state::ProviderModelsSummary],
    pub mcp_editor: &'a str,
    pub started: bool,
    pub acp: bool,
}
pub fn candidates(
    input: CandidateInput<'_>,
) -> desktop_session::message::chat::input::slash_palette::SlashPaletteCandidates {
    use desktop_session::message::chat::input::slash_palette::{
        SlashPaletteCandidate, SlashPaletteCandidates, mcp_candidates_from_editor,
    };
    let skills = input
        .skills
        .into_iter()
        .map(|mut skill| {
            if skill.label.trim().is_empty() {
                skill.label = skill.id.clone();
            }
            skill.description =
                skill.description.map(|d| d.trim().to_string()).filter(|d| !d.is_empty());
            skill
        })
        .collect();
    let tools = if input.acp {
        Vec::new()
    } else {
        input.tools.into_iter().filter(|tool| !tool.id.trim().is_empty()).collect()
    };
    let mcps = mcp_candidates_from_editor(input.mcp_editor)
        .into_iter()
        .map(|(_, _, candidate)| candidate)
        .collect();
    if input.started {
        return SlashPaletteCandidates { skills, tools, mcps, ..Default::default() };
    }
    let mut models = vec![SlashPaletteCandidate {
        id: "auto".into(),
        label: "Auto".into(),
        description: Some("自动选择模型".into()),
    }];
    for provider in input.providers {
        for model in provider.models.iter().filter(|model| model.enabled) {
            models.push(SlashPaletteCandidate {
                id: format!("{}/{}", provider.id, model.id),
                label: model.name.clone(),
                description: Some(provider.name.clone()),
            });
        }
    }
    SlashPaletteCandidates { skills, tools, mcps, models, acps: input.acps }
}

/// 快捷选择产生的跨域操作；执行权仍归对应领域。
pub enum SelectionEffect {
    Model(String),
    Agent(Option<String>),
    Tool(String),
    Skill(String),
    Mcp(String),
}
pub fn selection_effect(
    entry: desktop_session::message::chat::input::slash_palette::SlashPaletteEntry,
) -> SelectionEffect {
    match entry.kind {
        SlashPaletteKind::Model => SelectionEffect::Model(entry.id),
        SlashPaletteKind::Acp => {
            SelectionEffect::Agent(if entry.id.trim().is_empty() { None } else { Some(entry.id) })
        }
        SlashPaletteKind::Tool => SelectionEffect::Tool(entry.id),
        SlashPaletteKind::Skill => SelectionEffect::Skill(entry.id),
        SlashPaletteKind::Mcp => SelectionEffect::Mcp(entry.id),
    }
}
pub fn mcp_toggle(editor: &str, server_name: &str) -> Option<(usize, bool)> {
    desktop_session::message::chat::input::slash_palette::mcp_candidates_from_editor(editor)
        .into_iter()
        .find(|(_, _, candidate)| candidate.id == server_name)
        .map(|(index, enabled, _)| (index, !enabled))
}

impl State {
    pub fn refresh_file_cache(
        &mut self,
        file_index: &[String],
        project_path: Option<&str>,
        revision: u64,
    ) {
        use desktop_session::message::chat::input::file_search::{
            normalize_file_search_query, refresh_file_search_entries,
        };
        let query = normalize_file_search_query(&self.file_search_query);
        let Some(results) = refresh_file_search_entries(
            file_index,
            &query,
            project_path,
            revision,
            &self.file_search_cache_query,
            self.file_search_cache_project_path.as_deref(),
            self.file_search_cache_revision,
        ) else {
            return;
        };
        self.file_search_cache_query = query;
        self.file_search_cache_project_path = project_path.map(str::to_owned);
        self.file_search_cache_revision = revision;
        self.file_search_cache_entries = results;
    }
}

impl State {
    pub fn current_file(&self) -> Option<String> {
        self.file_search_cache_entries.get(self.file_search_selected_index).map(|entry| entry.path.clone())
    }
}
