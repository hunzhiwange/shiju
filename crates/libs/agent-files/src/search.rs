use agent_log as log;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use super::{SearchInput, ripgrep};

static LOG: LazyLock<log::Logger> = LazyLock::new(|| {
    log::create(Some({
        let mut map = Map::new();
        map.insert("service".to_string(), Value::String("file".to_string()));
        map
    }))
});

/// 搜索文件和目录。
pub fn search(root: impl AsRef<Path>, input: SearchInput) -> Vec<String> {
    let query = input.query.trim().to_string();
    let kind = input
        .r#type
        .as_deref()
        .map(|kind| kind.to_string())
        .unwrap_or_else(|| if input.dirs { "all".to_string() } else { "file".to_string() });

    LOG.info(
        "search",
        Some({
            let mut map = Map::new();
            map.insert("query".to_string(), Value::String(query.clone()));
            map.insert("kind".to_string(), Value::String(kind.clone()));
            map
        }),
    );

    let entry = scan_files_dirs(root.as_ref());
    let mut items = match kind.as_str() {
        "file" => entry.files,
        "directory" => entry.dirs,
        _ => {
            let mut items = entry.files;
            items.extend(entry.dirs);
            items
        }
    };

    if query.is_empty() {
        items.sort();
        items.truncate(input.limit);
        return items;
    }

    let mut scored = items
        .into_iter()
        .map(|item| {
            let score = fuzzy_score(&query, &item);
            (score, item)
        })
        .filter(|(score, _)| *score < i64::MAX)
        .collect::<Vec<_>>();

    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    scored.into_iter().take(input.limit).map(|(_, item)| item).collect()
}

#[derive(Debug, Clone)]
struct ScanEntry {
    files: Vec<String>,
    dirs: Vec<String>,
}

fn scan_files_dirs(root: &Path) -> ScanEntry {
    let mut files = Vec::new();
    let mut dirs_set = HashSet::<String>::new();

    let list = ripgrep::files(ripgrep::FilesInput {
        cwd: root.to_path_buf(),
        glob: None,
        hidden: Some(true),
        follow: Some(false),
        max_depth: None,
    })
    .unwrap_or_default();

    for file in list {
        files.push(file.clone());
        let mut current = PathBuf::from(&file);
        loop {
            let parent = current.parent().map(|path| path.to_path_buf());
            let Some(parent) = parent else {
                break;
            };
            if parent.as_os_str().is_empty() {
                break;
            }
            let dir = parent.to_string_lossy().to_string().replace('\\', "/");
            if dir == "." || dir.is_empty() {
                break;
            }
            dirs_set.insert(format!("{}/", dir.trim_end_matches('/')));
            current = parent;
        }
    }

    let mut dirs = dirs_set.into_iter().collect::<Vec<_>>();
    dirs.sort();
    files.sort();
    ScanEntry { files, dirs }
}

pub(super) fn is_hidden(item: &str) -> bool {
    let normalized = item.replace('\\', "/").trim_end_matches('/').to_string();
    normalized.split('/').any(|part| part.starts_with('.') && part.len() > 1)
}

pub(super) fn fuzzy_score(query: &str, candidate: &str) -> i64 {
    let query = query.to_ascii_lowercase();
    let candidate_lower = candidate.to_ascii_lowercase();

    if !is_hidden(candidate) && (query.starts_with('.') || query.contains("/.")) {
        return i64::MAX;
    }

    if candidate_lower.contains(&query) {
        return candidate_lower.len().saturating_sub(query.len()).saturating_sub(1) as i64;
    }

    strsim::levenshtein(&query, &candidate_lower) as i64
}
