//! 文件读取状态缓存。
//!
//! 本模块为 Claude Tools V2 运行时提供跨工具共享的文件读取状态，后续
//! `file_read`、`file_write`、`apply_patch` 等工具可以基于这里的快照判断：
//!
//! - 某个路径是否已经在本轮上下文中读过
//! - 读取是否只是局部视图（partial view）
//! - 读取缓存是否需要因写操作失效
//!
//! 当前实现保持最小但可直接复用：
//!
//! - 路径会做工作区感知的归一化
//! - 缓存采用 LRU 淘汰
//! - 受最大条目数与总字节数双重限制
//! - 支持克隆与 merge，方便在批量工具或并行执行中合并状态

use std::collections::{HashMap, VecDeque, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};

/// 默认最多保留的读取条目数。
const DEFAULT_MAX_ENTRIES: usize = 256;

/// 默认最多保留的读取字节数。
const DEFAULT_MAX_TOTAL_BYTES: usize = 1_048_576;

/// 文件内容快照。
///
/// 用于在 `edit` / `file_write` 等修改型工具执行前，判断当前磁盘内容是否仍与最近
/// 一次 `file_read` 后看到的版本一致。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSnapshot {
    /// 文件大小（字节）。
    pub size_bytes: u64,
    /// 内容摘要。
    pub content_digest: u64,
}

impl FileSnapshot {
    /// 从原始字节构造稳定快照。
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        Self { size_bytes: bytes.len() as u64, content_digest: hasher.finish() }
    }

    /// 从 UTF-8 文本构造快照。
    pub fn from_text(text: &str) -> Self {
        Self::from_bytes(text.as_bytes())
    }

    /// 判断给定文本是否与当前快照一致。
    pub fn matches_text(&self, text: &str) -> bool {
        self == &Self::from_text(text)
    }
}

/// 单个文件读取状态条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReadStateEntry {
    /// 归一化后的文件路径。
    pub path: PathBuf,
    /// 最近一次读取所对应的字节数估计。
    pub bytes_read: usize,
    /// 最近一次读取是否为局部视图。
    pub partial_view: bool,
    /// 最近一次读取请求的起始偏移。
    pub offset: Option<usize>,
    /// 最近一次读取请求的范围限制。
    pub limit: Option<usize>,
    /// 最近一次读取对应的文件内容快照。
    pub snapshot: Option<FileSnapshot>,
}

/// 文件读取状态缓存。
#[derive(Debug, Clone)]
pub struct FileReadStateCache {
    max_entries: usize,
    max_total_bytes: usize,
    total_bytes: usize,
    entries: HashMap<PathBuf, FileReadStateEntry>,
    lru: VecDeque<PathBuf>,
}

impl Default for FileReadStateCache {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ENTRIES, DEFAULT_MAX_TOTAL_BYTES)
    }
}

impl FileReadStateCache {
    /// 创建新的读取状态缓存。
    pub fn new(max_entries: usize, max_total_bytes: usize) -> Self {
        Self {
            max_entries: max_entries.max(1),
            max_total_bytes: max_total_bytes.max(1),
            total_bytes: 0,
            entries: HashMap::new(),
            lru: VecDeque::new(),
        }
    }

    /// 返回当前缓存条目数。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 返回当前缓存的总字节数。
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// 判断缓存是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 清空缓存。
    pub fn clear(&mut self) {
        self.entries.clear();
        self.lru.clear();
        self.total_bytes = 0;
    }

    /// 获取指定路径的读取状态，并将其刷新到 LRU 末尾。
    pub fn get(
        &mut self,
        root: Option<&Path>,
        path: impl AsRef<Path>,
    ) -> Option<FileReadStateEntry> {
        let normalized = Self::normalized_path(root, path);
        let entry = self.entries.get(&normalized).cloned();
        if entry.is_some() {
            self.touch(&normalized);
        }
        entry
    }

    /// 记录一次读取行为。
    pub fn note_read(
        &mut self,
        root: Option<&Path>,
        path: impl AsRef<Path>,
        bytes_read: usize,
        partial_view: bool,
        offset: Option<usize>,
        limit: Option<usize>,
        snapshot: Option<FileSnapshot>,
    ) -> PathBuf {
        let normalized = Self::normalized_path(root, path);
        let entry = FileReadStateEntry {
            path: normalized.clone(),
            bytes_read,
            partial_view,
            offset,
            limit,
            snapshot,
        };
        self.insert_entry(entry);
        normalized
    }

    /// 使某个路径的缓存失效。
    pub fn invalidate(
        &mut self,
        root: Option<&Path>,
        path: impl AsRef<Path>,
    ) -> Option<FileReadStateEntry> {
        let normalized = Self::normalized_path(root, path);
        self.remove_entry(&normalized)
    }

    /// 将另一个缓存的状态合并进当前缓存。
    pub fn merge(&mut self, other: &Self) {
        for path in &other.lru {
            if let Some(entry) = other.entries.get(path).cloned() {
                self.insert_entry(entry);
            }
        }
    }

    /// 计算用于缓存键的归一化路径。
    pub fn normalized_path(root: Option<&Path>, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        let joined = if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(root) = root {
            root.join(path)
        } else {
            path.to_path_buf()
        };

        joined.canonicalize().unwrap_or_else(|_| normalize_lexically(&joined))
    }

    fn insert_entry(&mut self, entry: FileReadStateEntry) {
        if let Some(old) = self.entries.insert(entry.path.clone(), entry.clone()) {
            self.total_bytes = self.total_bytes.saturating_sub(old.bytes_read);
        }

        self.total_bytes = self.total_bytes.saturating_add(entry.bytes_read);
        self.touch(&entry.path);
        self.evict_if_needed();
    }

    fn remove_entry(&mut self, path: &Path) -> Option<FileReadStateEntry> {
        self.lru.retain(|candidate| candidate != path);
        let removed = self.entries.remove(path);
        if let Some(entry) = &removed {
            self.total_bytes = self.total_bytes.saturating_sub(entry.bytes_read);
        }
        removed
    }

    fn touch(&mut self, path: &Path) {
        self.lru.retain(|candidate| candidate != path);
        self.lru.push_back(path.to_path_buf());
    }

    fn evict_if_needed(&mut self) {
        while self.entries.len() > self.max_entries || self.total_bytes > self.max_total_bytes {
            let Some(oldest) = self.lru.pop_front() else {
                break;
            };
            if let Some(entry) = self.entries.remove(&oldest) {
                self.total_bytes = self.total_bytes.saturating_sub(entry.bytes_read);
            }
        }
    }
}

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new("/")),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
        }
    }

    if normalized.as_os_str().is_empty() { PathBuf::from(".") } else { normalized }
}
#[cfg(test)]
mod tests;
