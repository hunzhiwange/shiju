use std::path::{Component, Path};

use super::super::effect::TuiEffect;
use super::super::model::search::{SearchResult, SearchState};

pub(crate) trait SearchProvider {
    fn search(&mut self, query: &str, generation: u64) -> Result<Vec<SearchResult>, String>;
}

pub(crate) fn run_search(
    state: &mut SearchState,
    query: impl Into<String>,
    provider: &mut dyn SearchProvider,
) -> u64 {
    let generation = state.begin_request(query);
    let result = provider.search(&state.query, generation);
    state.apply_results(generation, result);
    generation
}

pub(crate) fn validated_open_effect(
    workspace: &Path,
    relative: &Path,
    line: Option<usize>,
) -> Result<TuiEffect, String> {
    if relative.is_absolute() || has_sensitive_component(relative) {
        return Err("search target is outside the allowed workspace surface".to_string());
    }
    let workspace = workspace
        .canonicalize()
        .map_err(|error| format!("workspace unavailable: {error}"))?;
    let target = workspace
        .join(relative)
        .canonicalize()
        .map_err(|error| format!("search target unavailable: {error}"))?;
    if !target.starts_with(&workspace) || !target.is_file() {
        return Err("search target escaped workspace or is not a file".to_string());
    }
    Ok(TuiEffect::OpenWorkspaceFile { path: target, line })
}

pub(crate) fn search_dialog_lines(state: &SearchState, width: usize) -> Vec<String> {
    let width = width.max(12);
    let mut lines = vec![format!("{:?} /{}", state.mode, state.query)];
    if let Some(error) = &state.error {
        lines.push(format!("error: {}", truncate(error, width)));
    } else if state.results.is_empty() {
        lines.push("No results".to_string());
    } else {
        lines.extend(state.results.iter().enumerate().map(|(index, result)| {
            truncate(
                &format!(
                    "{} {} · {}",
                    if index == state.selected { ">" } else { " " },
                    result.label,
                    result.preview
                ),
                width,
            )
        }));
    }
    lines
}

fn has_sensitive_component(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::ParentDir | Component::RootDir | Component::Prefix(_) => true,
        Component::Normal(value) => {
            let value = value.to_string_lossy().to_ascii_lowercase();
            value == ".git"
                || value == ".env"
                || value.ends_with(".pem")
                || value.ends_with(".key")
        }
        Component::CurDir => false,
    })
}

fn truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_string();
    }
    value.chars().take(width.saturating_sub(1)).collect::<String>() + "…"
}
