//! 构建和读取项目文件索引。
//! 本模块用受限深度、数量上限和缓存校验控制文件系统扫描成本。

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

const FILE_INDEX_LIMIT: usize = 300_000usize;
const FILE_INDEX_MAX_DEPTH: usize = 40usize;
const FILE_INDEX_CACHE_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct FileMtime {
    secs: u64,
    nanos: u32,
}

impl FileMtime {
    #[cfg(not(target_arch = "wasm32"))]
    fn from_system_time(time: SystemTime) -> Option<Self> {
        let duration = time.duration_since(UNIX_EPOCH).ok()?;
        Some(Self { secs: duration.as_secs(), nanos: duration.subsec_nanos() })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DirEntry {
    path: String,
    mtime: Option<FileMtime>,
    files: Vec<String>,
    subdirs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileIndexCache {
    version: u8,
    root: String,
    root_mtime: Option<FileMtime>,
    gitignore_mtime: Option<FileMtime>,
    entries: Vec<DirEntry>,
}

/// 公开结构体，承载 FileIndexLoadResult 对应的状态数据。
/// 字段保持与相邻业务流程和序列化格式一致。
#[derive(Debug, Clone, Default)]
pub struct FileIndexLoadResult {
    pub files: Vec<String>,
    pub needs_refresh: bool,
}

fn dir_mtime(path: &Path) -> Option<FileMtime> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::metadata(path)
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(FileMtime::from_system_time)
    }
}

fn gitignore_mtime(root: &Path) -> Option<FileMtime> {
    dir_mtime(&root.join(".gitignore"))
}

fn hex_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input.as_bytes() {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn file_index_cache_path(root: &str, cache_dir: Option<&Path>) -> Option<PathBuf> {
    Some(cache_dir?.join(format!("{}.json", hex_encode(root))))
}

fn load_file_index_cache(root: &str, cache_dir: Option<&Path>) -> Option<FileIndexCache> {
    let path = file_index_cache_path(root, cache_dir)?;
    let content = std::fs::read_to_string(path).ok()?;
    let cache = serde_json::from_str::<FileIndexCache>(&content).ok()?;
    if cache.version != FILE_INDEX_CACHE_VERSION || cache.root != root {
        return None;
    }
    Some(cache)
}

fn save_file_index_cache(root: &str, cache_dir: Option<&Path>, cache: &FileIndexCache) {
    let Some(path) = file_index_cache_path(root, cache_dir) else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let Ok(content) = serde_json::to_string(cache) else {
        return;
    };
    let _ = std::fs::write(path, content);
}

fn cache_map(cache: &FileIndexCache) -> HashMap<String, DirEntry> {
    cache.entries.iter().cloned().map(|entry| (entry.path.clone(), entry)).collect()
}

fn collect_cached_subtree(
    rel_dir: &str,
    cache: &HashMap<String, DirEntry>,
    files_out: &mut Vec<String>,
    entries_out: &mut Vec<DirEntry>,
) {
    let Some(entry) = cache.get(rel_dir) else {
        return;
    };
    entries_out.push(entry.clone());
    for file in &entry.files {
        if files_out.len() >= FILE_INDEX_LIMIT {
            return;
        }
        files_out.push(file.clone());
    }
    for subdir in &entry.subdirs {
        if files_out.len() >= FILE_INDEX_LIMIT {
            return;
        }
        collect_cached_subtree(subdir, cache, files_out, entries_out);
    }
}

fn cache_files(cache: &FileIndexCache) -> Vec<String> {
    let map = cache_map(cache);
    let mut files = Vec::new();
    let mut entries = Vec::new();
    collect_cached_subtree("", &map, &mut files, &mut entries);
    files
}

/// 读取指定缓存目录中的项目索引；未提供目录时请求刷新。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn load_file_index(root: &str, cache_dir: Option<&Path>) -> FileIndexLoadResult {
    let Some(cache) = load_file_index_cache(root, cache_dir) else {
        return FileIndexLoadResult { files: Vec::new(), needs_refresh: true };
    };
    let current_gitignore = gitignore_mtime(Path::new(root));
    if cache.gitignore_mtime != current_gitignore {
        return FileIndexLoadResult { files: Vec::new(), needs_refresh: true };
    }
    FileIndexLoadResult { files: cache_files(&cache), needs_refresh: true }
}

/// 刷新项目索引；仅在提供缓存目录时读写缓存。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn refresh_file_index(root: &str, cache_dir: Option<&Path>) -> Vec<String> {
    let cache = load_file_index_cache(root, cache_dir);
    let cache = cache.filter(|item| item.gitignore_mtime == gitignore_mtime(Path::new(root)));
    let updated = index_files_with_cache(root, cache.as_ref());
    let files = cache_files(&updated);
    save_file_index_cache(root, cache_dir, &updated);
    files
}

/// 公开函数，执行 index_files 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn index_files(root: String) -> Vec<String> {
    #[derive(Clone)]
    struct IgnoreRule {
        negated: bool,
        directory_only: bool,
        anchored: bool,
        has_slash: bool,
        exact: String,
        glob: Option<glob::Pattern>,
    }

    impl IgnoreRule {
        fn matches(&self, rel: &str, name: &str, is_dir: bool) -> bool {
            if self.directory_only && !is_dir {
                return false;
            }

            if let Some(glob) = &self.glob {
                if self.has_slash || self.anchored { glob.matches(rel) } else { glob.matches(name) }
            } else if self.has_slash || self.anchored {
                rel == self.exact
            } else {
                name == self.exact
            }
        }
    }

    fn parse_ignore_rule(line: &str) -> Option<IgnoreRule> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }

        let (negated, raw) =
            trimmed.strip_prefix('!').map_or((false, trimmed), |item| (true, item.trim()));
        if raw.is_empty() || raw.starts_with('#') {
            return None;
        }

        let normalized = raw.replace('\\', "/");
        let anchored = normalized.starts_with('/');
        let directory_only = normalized.ends_with('/');
        let exact = normalized.trim_start_matches('/').trim_end_matches('/').to_string();
        if exact.is_empty() {
            return None;
        }

        let has_slash = exact.contains('/');
        let has_glob = exact.contains('*') || exact.contains('?') || exact.contains('[');
        let glob = has_glob.then(|| glob::Pattern::new(&exact).ok()).flatten();

        Some(IgnoreRule { negated, directory_only, anchored, has_slash, exact, glob })
    }

    fn load_gitignore_rules(root: &str) -> Vec<IgnoreRule> {
        let path = std::path::Path::new(root).join(".gitignore");
        let Ok(content) = std::fs::read_to_string(path) else {
            return Vec::new();
        };

        content.lines().filter_map(parse_ignore_rule).collect()
    }

    let limit = FILE_INDEX_LIMIT;
    let max_depth = FILE_INDEX_MAX_DEPTH;
    let mut ignore_rules = [
        ".git/",
        "target/",
        "node_modules/",
        "dist/",
        "build/",
        "coverage/",
        ".cache/",
        ".next/",
        "out/",
        ".idea/",
        ".vscode/",
    ]
    .into_iter()
    .filter_map(parse_ignore_rule)
    .collect::<Vec<_>>();
    ignore_rules.extend(load_gitignore_rules(&root));

    let mut out = Vec::new();
    let mut queue = VecDeque::new();
    queue.push_back((std::path::PathBuf::from(&root), 0usize));

    while let Some((dir, depth)) = queue.pop_front() {
        if out.len() >= limit || depth > max_depth {
            break;
        }
        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            continue;
        };

        let mut entries = read_dir.flatten().collect::<Vec<_>>();
        entries.sort_by_key(|entry| {
            let name = entry.file_name().to_string_lossy().to_string().to_lowercase();
            let is_dir = entry.path().is_dir();
            (if is_dir { 0usize } else { 1usize }, name)
        });

        for entry in entries {
            if out.len() >= limit {
                break;
            }
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let rel =
                path.strip_prefix(&root).ok().map(|item| item.to_string_lossy().replace('\\', "/"));
            let Some(rel) = rel else {
                continue;
            };

            let is_dir = path.is_dir();
            let ignored = ignore_rules
                .iter()
                .filter(|rule| rule.matches(&rel, name.as_ref(), is_dir))
                .fold(false, |_, rule| !rule.negated);

            if ignored {
                continue;
            }

            if is_dir {
                queue.push_back((path, depth + 1));
                continue;
            }

            if let Some(p) = path.to_str() {
                out.push(p.to_string());
            }
        }
    }

    out
}

fn index_files_with_cache(root: &str, cache: Option<&FileIndexCache>) -> FileIndexCache {
    #[derive(Clone)]
    struct IgnoreRule {
        negated: bool,
        directory_only: bool,
        anchored: bool,
        has_slash: bool,
        exact: String,
        glob: Option<glob::Pattern>,
    }

    impl IgnoreRule {
        fn matches(&self, rel: &str, name: &str, is_dir: bool) -> bool {
            if self.directory_only && !is_dir {
                return false;
            }

            if let Some(glob) = &self.glob {
                if self.has_slash || self.anchored { glob.matches(rel) } else { glob.matches(name) }
            } else if self.has_slash || self.anchored {
                rel == self.exact
            } else {
                name == self.exact
            }
        }
    }

    fn parse_ignore_rule(line: &str) -> Option<IgnoreRule> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }

        let (negated, raw) =
            trimmed.strip_prefix('!').map_or((false, trimmed), |item| (true, item.trim()));
        if raw.is_empty() || raw.starts_with('#') {
            return None;
        }

        let normalized = raw.replace('\\', "/");
        let anchored = normalized.starts_with('/');
        let directory_only = normalized.ends_with('/');
        let exact = normalized.trim_start_matches('/').trim_end_matches('/').to_string();
        if exact.is_empty() {
            return None;
        }

        let has_slash = exact.contains('/');
        let has_glob = exact.contains('*') || exact.contains('?') || exact.contains('[');
        let glob = has_glob.then(|| glob::Pattern::new(&exact).ok()).flatten();

        Some(IgnoreRule { negated, directory_only, anchored, has_slash, exact, glob })
    }

    fn load_gitignore_rules(root: &str) -> Vec<IgnoreRule> {
        let path = std::path::Path::new(root).join(".gitignore");
        let Ok(content) = std::fs::read_to_string(path) else {
            return Vec::new();
        };

        content.lines().filter_map(parse_ignore_rule).collect()
    }

    fn rel_path(root: &Path, path: &Path) -> Option<String> {
        path.strip_prefix(root).ok().map(|item| item.to_string_lossy().replace('\\', "/"))
    }

    fn scan_dir(
        root: &Path,
        dir: &Path,
        rel_dir: &str,
        depth: usize,
        cache_map: Option<&HashMap<String, DirEntry>>,
        ignore_rules: &[IgnoreRule],
        files_out: &mut Vec<String>,
        entries_out: &mut Vec<DirEntry>,
    ) {
        if files_out.len() >= FILE_INDEX_LIMIT || depth > FILE_INDEX_MAX_DEPTH {
            return;
        }

        let mtime = dir_mtime(dir);
        if let Some(cache_map) = cache_map
            && let Some(cached) = cache_map.get(rel_dir)
            && cached.mtime == mtime
        {
            collect_cached_subtree(rel_dir, cache_map, files_out, entries_out);
            return;
        }

        let Ok(read_dir) = std::fs::read_dir(dir) else {
            return;
        };

        let mut entries = read_dir.flatten().collect::<Vec<_>>();
        entries.sort_by_key(|entry| {
            let name = entry.file_name().to_string_lossy().to_string().to_lowercase();
            let is_dir = entry.path().is_dir();
            (if is_dir { 0usize } else { 1usize }, name)
        });

        let mut files = Vec::new();
        let mut subdirs = Vec::new();

        for entry in entries {
            if files_out.len() >= FILE_INDEX_LIMIT {
                break;
            }
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let rel = rel_path(root, &path);
            let Some(rel) = rel else {
                continue;
            };

            let is_dir = path.is_dir();
            let ignored = ignore_rules
                .iter()
                .filter(|rule| rule.matches(&rel, name.as_ref(), is_dir))
                .fold(false, |_, rule| !rule.negated);

            if ignored {
                continue;
            }

            if is_dir {
                subdirs.push(rel.clone());
                scan_dir(
                    root,
                    &path,
                    &rel,
                    depth + 1,
                    cache_map,
                    ignore_rules,
                    files_out,
                    entries_out,
                );
                continue;
            }

            if let Some(p) = path.to_str() {
                let file = p.to_string();
                files.push(file.clone());
                files_out.push(file);
            }
        }

        entries_out.push(DirEntry { path: rel_dir.to_string(), mtime, files, subdirs });
    }

    let mut ignore_rules = [
        ".git/",
        "target/",
        "node_modules/",
        "dist/",
        "build/",
        "coverage/",
        ".cache/",
        ".next/",
        "out/",
        ".idea/",
        ".vscode/",
    ]
    .into_iter()
    .filter_map(parse_ignore_rule)
    .collect::<Vec<_>>();
    ignore_rules.extend(load_gitignore_rules(root));

    let root_path = Path::new(root);
    let gitignore = gitignore_mtime(root_path);
    let cache_map = cache.filter(|item| item.gitignore_mtime == gitignore).map(cache_map);

    let mut files = Vec::new();
    let mut entries = Vec::new();
    scan_dir(
        root_path,
        root_path,
        "",
        0,
        cache_map.as_ref(),
        &ignore_rules,
        &mut files,
        &mut entries,
    );

    FileIndexCache {
        version: FILE_INDEX_CACHE_VERSION,
        root: root.to_string(),
        root_mtime: dir_mtime(root_path),
        gitignore_mtime: gitignore,
        entries,
    }
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;

#[cfg(test)]
mod host_module_43_tests;
