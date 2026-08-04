use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use vw_agent::file::ripgrep::{FilesInput, files};

use super::commands::registry::CommandRegistry;
use super::input::{TuiPromptSuggestion, slash_prompt_suggestions};
use super::model::integration::{
    IntegrationKind, IntegrationLoadState, IntegrationPanel, sanitize_integration_text,
};
use super::model::overlay::{UiCommandPaletteOverlay, UiPaletteItem};
use super::state::TuiState;

const SLASH_PALETTE_VISIBLE_ROWS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum CompletionKind {
    SlashCommand,
    Directory,
    File,
    Agent,
    Skill,
    McpResource,
    Keyword,
}

impl CompletionKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::SlashCommand => "命令",
            Self::Directory => "目录",
            Self::File => "文件",
            Self::Agent => "Agent",
            Self::Skill => "Skill",
            Self::McpResource => "MCP",
            Self::Keyword => "关键字",
        }
    }

    fn rank(self) -> usize {
        match self {
            Self::SlashCommand => 0,
            Self::Directory => 1,
            Self::File => 2,
            Self::Agent => 3,
            Self::Skill => 4,
            Self::McpResource => 5,
            Self::Keyword => 6,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletionCatalogItem {
    pub(crate) replacement: String,
    pub(crate) label: String,
    pub(crate) detail: Option<String>,
    pub(crate) kind: CompletionKind,
    pub(crate) score: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CompletionCatalog {
    pub(crate) items: Vec<CompletionCatalogItem>,
    pub(crate) added_directories: Vec<PathBuf>,
    pub(crate) source_errors: BTreeMap<CompletionKind, String>,
    pub(crate) file_index: FileCompletionIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FileCompletionIndex {
    pub(crate) roots: Vec<PathBuf>,
    pub(crate) revision: u64,
    entries: Vec<FileCompletionEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FileCompletionEntry {
    root: PathBuf,
    relative: String,
    is_directory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletionQuery {
    pub(crate) raw: String,
    pub(crate) token_start: usize,
    pub(crate) token: String,
    pub(crate) kinds: Vec<CompletionKind>,
}

pub(crate) trait CompletionProvider {
    fn complete(
        &self,
        query: &CompletionQuery,
        state: &TuiState,
    ) -> Result<Vec<TuiPromptSuggestion>, String>;
}

pub(crate) fn completion_suggestions(state: &TuiState) -> Vec<TuiPromptSuggestion> {
    let Some(query) = parse_completion_query(state.prompt.value.as_str()) else {
        return Vec::new();
    };
    let providers: [&dyn CompletionProvider; 3] =
        [&SlashCompletionProvider, &FileCompletionProvider, &CatalogCompletionProvider];
    let mut suggestions = providers
        .iter()
        .flat_map(|provider| provider.complete(&query, state).unwrap_or_default())
        .collect::<Vec<_>>();
    let mut seen = HashSet::new();
    suggestions.sort_by(compare_suggestions);
    suggestions.retain(|suggestion| seen.insert(suggestion.replacement.clone()));
    suggestions.truncate(32);
    suggestions
}

pub(crate) fn completion_notices(state: &TuiState) -> Vec<String> {
    state
        .completion_catalog
        .source_errors
        .iter()
        .map(|(kind, error)| format!("{} 补全暂不可用: {error}", kind.label()))
        .collect()
}

pub(crate) fn sync_skill_completion_catalog(
    catalog: &mut CompletionCatalog,
    panel: &IntegrationPanel,
) {
    if panel.kind != IntegrationKind::Skill {
        return;
    }
    match &panel.state {
        IntegrationLoadState::Loading => return,
        IntegrationLoadState::Error(error) => {
            catalog.source_errors.insert(
                CompletionKind::Skill,
                sanitize_integration_text(error),
            );
            return;
        }
        IntegrationLoadState::Empty | IntegrationLoadState::Ready => {}
    }

    catalog.items.retain(|item| item.kind != CompletionKind::Skill);
    catalog.source_errors.remove(&CompletionKind::Skill);
    let mut seen = HashSet::new();
    catalog.items.extend(panel.items.iter().filter_map(|item| {
        let id = item.id.trim();
        let safe_id = !id.is_empty()
            && id
                .chars()
                .all(|ch| !ch.is_whitespace() && !ch.is_control() && ch != '$');
        if !item.enabled || !safe_id || !seen.insert(id.to_string()) {
            return None;
        }
        Some(CompletionCatalogItem {
            replacement: format!("${id}"),
            label: if item.title.trim().is_empty() {
                id.to_string()
            } else {
                sanitize_integration_text(item.title.as_str())
            },
            detail: (!item.detail.trim().is_empty())
                .then(|| sanitize_integration_text(item.detail.as_str())),
            kind: CompletionKind::Skill,
            score: 100,
        })
    }));
}

pub(crate) fn refresh_file_completion_index(
    catalog: &mut CompletionCatalog,
    workspace_root: Option<&Path>,
) {
    let mut candidates = workspace_root
        .into_iter()
        .map(Path::to_path_buf)
        .chain(catalog.added_directories.iter().cloned())
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.dedup();

    let mut roots = Vec::new();
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    for candidate in candidates {
        let root = match candidate.canonicalize() {
            Ok(root) if root.is_dir() => root,
            Ok(_) => {
                errors.push(format!("补全根目录不是目录: {}", candidate.display()));
                continue;
            }
            Err(error) => {
                errors.push(format!("无法读取补全根目录 {}: {error}", candidate.display()));
                continue;
            }
        };
        if roots.contains(&root) {
            continue;
        }
        roots.push(root.clone());
        match files(FilesInput {
            cwd: root.clone(),
            glob: None,
            hidden: Some(false),
            follow: Some(false),
            max_depth: Some(6),
        }) {
            Ok(relative_files) => {
                for relative in relative_files {
                    entries.push(FileCompletionEntry {
                        root: root.clone(),
                        relative: relative.clone(),
                        is_directory: false,
                    });
                    let mut parent = Path::new(relative.as_str()).parent();
                    while let Some(directory) = parent {
                        if directory.as_os_str().is_empty() {
                            break;
                        }
                        entries.push(FileCompletionEntry {
                            root: root.clone(),
                            relative: directory.to_string_lossy().replace('\\', "/"),
                            is_directory: true,
                        });
                        parent = directory.parent();
                    }
                }
            }
            Err(error) => {
                errors.push(format!("无法索引补全根目录 {}: {error}", root.display()));
            }
        }
    }
    roots.sort();
    entries.sort();
    entries.dedup();
    catalog.file_index = FileCompletionIndex {
        roots,
        revision: catalog.file_index.revision.saturating_add(1),
        entries,
    };
    if errors.is_empty() {
        catalog.source_errors.remove(&CompletionKind::File);
    } else {
        catalog.source_errors.insert(CompletionKind::File, errors.join("; "));
    }
}

pub(crate) fn slash_palette_overlay(
    registry: &CommandRegistry,
    query: &str,
) -> UiCommandPaletteOverlay {
    let query = query.trim().trim_start_matches('/').to_string();
    let items = registry
        .matching_commands(query.as_str())
        .into_iter()
        .map(|spec| {
            let alias_detail =
                (!spec.aliases.is_empty()).then(|| format!("别名: /{}", spec.aliases.join(", /")));
            UiPaletteItem {
                id: spec.name.clone(),
                label: format!("/{}", spec.name),
                detail: alias_detail
                    .map(|aliases| format!("{} · {aliases}", spec.summary))
                    .or_else(|| Some(spec.summary.clone())),
                source: format!("{:?}", spec.source),
                argument_hint: spec.argument_hint.clone(),
                available: spec.availability.is_available(),
                unavailable_reason: spec.availability.unavailable_reason().map(str::to_string),
                replacement: format!(
                    "/{}{}",
                    spec.name,
                    if spec.takes_argument { " " } else { "" }
                ),
            }
        })
        .collect::<Vec<_>>();
    let selected_index = (!items.is_empty()).then_some(0);
    UiCommandPaletteOverlay {
        query,
        items,
        selected_index,
        first_visible_index: 0,
        visible_rows: SLASH_PALETTE_VISIBLE_ROWS,
        notice: None,
    }
}

pub(crate) fn directory_command_suggestions(
    state: &TuiState,
    current: &str,
) -> Vec<TuiPromptSuggestion> {
    let root = state
        .project
        .workspace_root
        .as_deref()
        .or_else(|| state.completion_catalog.added_directories.first().map(PathBuf::as_path));
    let Some(root) = root.and_then(|path| path.canonicalize().ok()) else {
        return Vec::new();
    };
    let current = current.trim().trim_matches('"');
    let requested = Path::new(current);
    let (parent, prefix) = if current.ends_with(std::path::MAIN_SEPARATOR) {
        (requested, "")
    } else {
        (
            requested.parent().unwrap_or_else(|| Path::new("")),
            requested.file_name().and_then(|name| name.to_str()).unwrap_or(""),
        )
    };
    let directory = if parent.is_absolute() {
        parent.to_path_buf()
    } else {
        root.join(parent)
    };
    let Ok(canonical_parent) = directory.canonicalize() else {
        return Vec::new();
    };
    if !canonical_parent.starts_with(&root) {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(&canonical_parent) else {
        return Vec::new();
    };
    let mut suggestions = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let metadata = fs::symlink_metadata(entry.path()).ok()?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return None;
            }
            let name = entry.file_name();
            let name = name.to_str()?;
            if name.starts_with('.') || !name.to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase()) {
                return None;
            }
            let canonical = entry.path().canonicalize().ok()?;
            if !canonical.starts_with(&root) {
                return None;
            }
            let relative = canonical.strip_prefix(&root).ok()?.to_string_lossy().replace('\\', "/");
            let argument = if relative.contains(' ') {
                format!("\"{relative}\"")
            } else {
                relative.clone()
            };
            Some(TuiPromptSuggestion {
                replacement: format!("/cd {argument}/"),
                label: format!("{relative}/"),
                detail: Some(root.display().to_string()),
                kind: CompletionKind::Directory,
                score: fuzzy_score(name, prefix) + 20,
                continue_completion: true,
            })
        })
        .collect::<Vec<_>>();
    suggestions.sort_by(compare_suggestions);
    suggestions.truncate(32);
    suggestions
}

pub(crate) fn active_completion_token(value: &str) -> Option<(usize, &str)> {
    if value.is_empty() || value.chars().next_back().is_some_and(char::is_whitespace) {
        return None;
    }
    let start = value
        .char_indices()
        .rev()
        .find_map(|(index, ch)| ch.is_whitespace().then_some(index + ch.len_utf8()))
        .unwrap_or_default();
    Some((start, &value[start..]))
}

fn parse_completion_query(value: &str) -> Option<CompletionQuery> {
    let raw = value.to_string();
    let trimmed = value.trim_start();
    let (token_start, token, kinds) = if trimmed.starts_with('/') {
        (
            value.len().saturating_sub(trimmed.len()),
            trimmed.to_string(),
            vec![CompletionKind::SlashCommand],
        )
    } else {
        let (token_start, token) = active_completion_token(value)?;
        if let Some(token) = token.strip_prefix('/') {
            (token_start, token.to_string(), vec![CompletionKind::SlashCommand])
        } else if let Some(token) = token.strip_prefix('@') {
            (
                token_start,
                token.to_string(),
                vec![CompletionKind::Directory, CompletionKind::File, CompletionKind::Agent],
            )
        } else if let Some(token) = token.strip_prefix('$') {
            (token_start, token.to_string(), vec![CompletionKind::Skill])
        } else if let Some(token) = token.strip_prefix("mcp:") {
            (token_start, token.to_string(), vec![CompletionKind::McpResource])
        } else if let Some(token) = token.strip_prefix('#') {
            (token_start, token.to_string(), vec![CompletionKind::Keyword])
        } else {
            return None;
        }
    };
    Some(CompletionQuery { raw, token_start, token, kinds })
}

struct SlashCompletionProvider;

impl CompletionProvider for SlashCompletionProvider {
    fn complete(
        &self,
        query: &CompletionQuery,
        state: &TuiState,
    ) -> Result<Vec<TuiPromptSuggestion>, String> {
        if !query.kinds.contains(&CompletionKind::SlashCommand) {
            return Ok(Vec::new());
        }
        Ok(slash_prompt_suggestions(state))
    }
}

struct CatalogCompletionProvider;

impl CompletionProvider for CatalogCompletionProvider {
    fn complete(
        &self,
        query: &CompletionQuery,
        state: &TuiState,
    ) -> Result<Vec<TuiPromptSuggestion>, String> {
        let query_lower = query.token.to_lowercase();
        Ok(state
            .completion_catalog
            .items
            .iter()
            .filter(|item| query.kinds.contains(&item.kind))
            .filter(|item| {
                query_lower.is_empty()
                    || item.label.to_lowercase().contains(query_lower.as_str())
                    || item.replacement.to_lowercase().contains(query_lower.as_str())
            })
            .map(|item| TuiPromptSuggestion {
                replacement: replace_current_token(query, item.replacement.as_str()),
                label: item.label.clone(),
                detail: item.detail.clone(),
                kind: item.kind,
                score: item.score + fuzzy_score(item.label.as_str(), query.token.as_str()),
                continue_completion: item.kind == CompletionKind::Directory,
            })
            .collect())
    }
}

struct FileCompletionProvider;

impl CompletionProvider for FileCompletionProvider {
    fn complete(
        &self,
        query: &CompletionQuery,
        state: &TuiState,
    ) -> Result<Vec<TuiPromptSuggestion>, String> {
        if !query
            .kinds
            .iter()
            .any(|kind| matches!(kind, CompletionKind::Directory | CompletionKind::File))
        {
            return Ok(Vec::new());
        }
        let query_lower = query.token.to_lowercase();
        Ok(state
            .completion_catalog
            .file_index
            .entries
            .iter()
            .filter(|entry| {
                query_lower.is_empty()
                    || entry.relative.to_lowercase().contains(query_lower.as_str())
            })
            .map(|entry| {
                let replacement_path = if entry.relative.contains(' ') {
                    format!("\"{}\"", entry.relative)
                } else {
                    entry.relative.clone()
                };
                let token =
                    format!("@{replacement_path}{}", if entry.is_directory { "/" } else { "" });
                TuiPromptSuggestion {
                    replacement: replace_current_token(query, token.as_str()),
                    label: token,
                    detail: Some(entry.root.display().to_string()),
                    kind: if entry.is_directory {
                        CompletionKind::Directory
                    } else {
                        CompletionKind::File
                    },
                    score: fuzzy_score(entry.relative.as_str(), query.token.as_str())
                        + if entry.is_directory { 20 } else { 10 },
                    continue_completion: entry.is_directory,
                }
            })
            .collect())
    }
}

fn replace_current_token(query: &CompletionQuery, replacement: &str) -> String {
    format!("{}{replacement}", &query.raw[..query.token_start])
}

fn compare_suggestions(left: &TuiPromptSuggestion, right: &TuiPromptSuggestion) -> Ordering {
    right
        .score
        .cmp(&left.score)
        .then_with(|| left.kind.rank().cmp(&right.kind.rank()))
        .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
        .then_with(|| left.replacement.cmp(&right.replacement))
}

fn fuzzy_score(candidate: &str, query: &str) -> i32 {
    if query.is_empty() {
        return 0;
    }
    let candidate = candidate.to_lowercase();
    let query = query.to_lowercase();
    if candidate == query {
        100
    } else if candidate.starts_with(query.as_str()) {
        60
    } else if candidate.contains(query.as_str()) {
        30
    } else {
        0
    }
}
