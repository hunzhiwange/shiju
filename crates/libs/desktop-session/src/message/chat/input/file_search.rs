use desktop_editor::components::input_mention_highlighter::{file_mention, is_mention_char};
use std::collections::BTreeSet;
#[derive(Debug, Clone)]
pub struct FileSearchResult {
    pub path: String,
    pub is_dir: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DroppedPath {
    pub path: String,
    pub is_dir: bool,
}
pub fn build_ranked_file_search_entries(
    file_index: &[String],
    query: &str,
) -> Vec<FileSearchResult> {
    let mut seen_dirs = BTreeSet::new();
    let mut candidates = Vec::new();

    for path in file_index {
        candidates.push(FileSearchResult { path: path.clone(), is_dir: false });

        let normalized = path.replace('\\', "/");
        let mut parts = normalized.split('/').filter(|part| !part.is_empty()).collect::<Vec<_>>();
        if parts.len() <= 1 {
            continue;
        }

        parts.pop();
        let mut prefix = String::new();
        for part in parts {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);

            let dir_path = format!("{}/", prefix);
            if seen_dirs.insert(dir_path.clone()) {
                candidates.push(FileSearchResult { path: dir_path, is_dir: true });
            }
        }
    }

    let mut ranked = candidates
        .iter()
        .filter_map(|entry| {
            let score = if query.is_empty() { 0 } else { file_match_score(&entry.path, query)? };
            Some((score, entry.is_dir, entry.path.clone()))
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)).then_with(|| a.2.cmp(&b.2)));
    ranked.into_iter().map(|(_, is_dir, path)| FileSearchResult { path, is_dir }).collect()
}

pub fn format_drop_mentions(
    project_root: Option<&str>,
    dropped_paths: &[DroppedPath],
    dropped_position: Option<(usize, usize)>,
) -> Vec<String> {
    let mention_position = if dropped_paths.len() == 1 { dropped_position } else { None };
    dropped_paths
        .iter()
        .map(|entry| {
            format_drop_mention_path(project_root, &entry.path, entry.is_dir, mention_position)
        })
        .collect()
}

fn format_drop_mention_path(
    project_root: Option<&str>,
    file_path: &str,
    is_dir: bool,
    position: Option<(usize, usize)>,
) -> String {
    let normalized = if let Some(project_root) = project_root
        && let Ok(rel_path) = std::path::Path::new(file_path).strip_prefix(project_root)
    {
        rel_path.to_string_lossy().replace('\\', "/")
    } else {
        file_path.replace('\\', "/")
    };
    let mention_path =
        if is_dir && !normalized.ends_with('/') { format!("{normalized}/") } else { normalized };

    match position {
        Some((line, col)) if !is_dir => format!("{mention_path}:{line}:{col}"),
        _ => mention_path,
    }
}

pub fn join_drop_mentions(mentions: &[String]) -> String {
    mentions.iter().map(|mention| format!("{} ", file_mention(mention))).collect()
}

pub fn normalized_lower_path(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

pub fn fuzzy_subsequence_score(candidate: &str, query: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }

    let cand = candidate.chars().collect::<Vec<_>>();
    let needle = query.chars().collect::<Vec<_>>();
    let mut score = 0i64;
    let mut cand_i = 0usize;
    let mut prev_match: Option<usize> = None;

    for q in needle {
        let mut found = None;
        while cand_i < cand.len() {
            if cand[cand_i] == q {
                found = Some(cand_i);
                break;
            }
            cand_i += 1;
        }

        let idx = found?;
        score += 10;

        if let Some(prev) = prev_match {
            if idx == prev + 1 {
                score += 8;
            } else {
                score -= (idx.saturating_sub(prev + 1) as i64).min(8);
            }
        }

        if idx == 0 {
            score += 15;
        } else if matches!(cand[idx - 1], '/' | '_' | '-' | '.' | ' ') {
            score += 5;
        }

        prev_match = Some(idx);
        cand_i = idx + 1;
    }

    Some(score)
}

pub fn file_match_score(path: &str, query: &str) -> Option<i64> {
    let normalized = normalized_lower_path(path);
    let file_name = normalized.rsplit('/').next().unwrap_or(&normalized);

    if query.is_empty() {
        return Some(0);
    }

    let full_subseq = fuzzy_subsequence_score(&normalized, query)?;
    let name_subseq = fuzzy_subsequence_score(file_name, query).unwrap_or(i64::MIN / 2);
    let mut score = full_subseq.max(name_subseq.saturating_add(120));

    if normalized == query {
        score += 2200;
    }
    if file_name == query {
        score += 2000;
    }
    if file_name.starts_with(query) {
        score += 800;
    }
    if let Some(pos) = file_name.find(query) {
        score += 600 - (pos as i64).min(200);
    }
    if normalized.starts_with(query) {
        score += 400;
    }
    if let Some(pos) = normalized.find(query) {
        score += 280 - (pos as i64).min(200);
    }

    Some(score)
}

pub fn is_file_search_char(c: char) -> bool {
    is_mention_char(c)
}

pub fn trailing_token(text: &str) -> &str {
    let mut token_start = 0usize;
    for (i, c) in text.char_indices() {
        if c.is_whitespace() {
            token_start = i + c.len_utf8();
        }
    }
    &text[token_start..]
}

pub fn active_file_search_query(text: &str) -> Option<&str> {
    let token = trailing_token(text);
    let at_index = token.rfind('@')?;
    let query = &token[at_index + 1..];
    query.chars().all(is_file_search_char).then_some(query)
}

pub fn replace_or_append_file_mention(text: &str, mention_with_at: &str) -> String {
    let token = trailing_token(text);
    let token_start = text.len().saturating_sub(token.len());
    let mention_with_space = format!("{} ", mention_with_at);

    if let Some(at_index) = token.rfind('@') {
        let query = &token[at_index + 1..];
        if query.chars().all(is_file_search_char) {
            let replace_start = token_start + at_index;
            let mut new_text = text.to_string();
            new_text.replace_range(replace_start.., &mention_with_space);
            return new_text;
        }
    }

    if token.starts_with('@') && token[1..].chars().all(is_file_search_char) {
        let mut new_text = text.to_string();
        new_text.replace_range(token_start.., &mention_with_space);
        return new_text;
    }

    let mut new_text = text.to_string();
    if !new_text.is_empty() && !new_text.ends_with(' ') {
        new_text.push(' ');
    }
    new_text.push_str(&mention_with_space);
    new_text
}
pub fn normalize_file_search_query(query: &str) -> String {
    query.trim().replace('\\', "/").to_ascii_lowercase()
}

/// Reuse the cached projection until its query, project or file index changes.
pub fn refresh_file_search_entries(
    files: &[String], query: &str, project: Option<&str>, revision: u64,
    cached_query: &str, cached_project: Option<&str>, cached_revision: u64,
) -> Option<Vec<FileSearchResult>> {
    if cached_query == query && cached_project == project && cached_revision == revision {
        return None;
    }
    Some(build_ranked_file_search_entries(files, query))
}
