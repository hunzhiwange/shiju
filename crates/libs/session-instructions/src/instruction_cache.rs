//! 按工作区隔离的指令解析缓存。
//!
//! 通过来源文件元数据判断缓存是否失效，避免复用已变化的指令集合。

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{InstructionSource, Resolved};

/// 指令本次被加载或复用的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadReason {
    Initial,
    CacheHit,
    FileChanged,
    Cleared,
    Compaction,
}

/// 指令重新加载后发出的只读事件详情。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionsLoaded {
    pub workspace: String,
    pub session: String,
    pub reason: LoadReason,
    pub sources: Vec<InstructionSource>,
    pub files: Vec<String>,
}

/// 指令加载观察者；回调失败仅记录诊断，不得改变解析结果。
pub trait InstructionsLoadedHook {
    /// 接收一次只读加载事件。
    fn instructions_loaded(&self, event: &InstructionsLoaded) -> Result<(), String>;
}

/// 以工作区、会话和入口文件为联合键的指令缓存。
#[derive(Debug, Default)]
pub struct InstructionCache {
    entries: HashMap<CacheKey, CacheEntry>,
    invalidations: HashMap<CacheKey, LoadReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    workspace: PathBuf,
    session: String,
    filepath: PathBuf,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    resolved: Option<Resolved>,
    files: Vec<FileState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileState {
    filepath: PathBuf,
    modified: Option<SystemTime>,
    len: Option<u64>,
}

impl InstructionCache {
    /// 解析入口文件；同工作区和会话的来源文件均未变化时复用缓存。
    pub fn load(
        &mut self,
        workspace: &Path,
        session: &str,
        filepath: &Path,
        loader: impl FnOnce() -> Option<Resolved>,
    ) -> (Option<Resolved>, LoadReason) {
        self.load_with_hook(workspace, session, filepath, loader, None)
    }

    /// 解析入口文件，并在实际重载后尽力发出只读事件。
    ///
    /// 钩子错误只写入告警，绝不使已成功的指令解析失败。
    pub fn load_with_hook(
        &mut self,
        workspace: &Path,
        session: &str,
        filepath: &Path,
        loader: impl FnOnce() -> Option<Resolved>,
        hook: Option<&dyn InstructionsLoadedHook>,
    ) -> (Option<Resolved>, LoadReason) {
        let key = CacheKey::new(workspace, session, filepath);
        if self.invalidations.get(&key).is_none()
            && let Some(entry) = self.entries.get(&key)
            && entry.is_current()
        {
            return (entry.resolved.clone(), LoadReason::CacheHit);
        }

        let reason = self.invalidations.remove(&key).unwrap_or_else(|| {
            if self.entries.contains_key(&key) {
                LoadReason::FileChanged
            } else {
                LoadReason::Initial
            }
        });
        let resolved = loader();
        let files = resolved_files(filepath, resolved.as_ref());
        let entry = CacheEntry {
            resolved: resolved.clone(),
            files: files.iter().map(|path| FileState::read(path)).collect(),
        };
        self.entries.insert(key.clone(), entry);

        if let Some(hook) = hook {
            let event = InstructionsLoaded {
                workspace: key.workspace.to_string_lossy().to_string(),
                session: key.session.clone(),
                reason,
                sources: resolved.as_ref().map(|value| value.sources.clone()).unwrap_or_default(),
                files: files.into_iter().map(|path| path.to_string_lossy().to_string()).collect(),
            };
            if let Err(error) = hook.instructions_loaded(&event) {
                tracing::warn!(%error, "instructions_loaded hook failed");
            }
        }

        (resolved, reason)
    }

    /// 在普通清理后把指定工作区和会话的全部缓存标记为过期。
    pub fn clear_session(&mut self, workspace: &Path, session: &str) {
        self.invalidate_session(workspace, session, LoadReason::Cleared);
    }

    /// 在上下文压缩后把指定工作区和会话的全部缓存标记为过期。
    pub fn invalidate_for_compaction(&mut self, workspace: &Path, session: &str) {
        self.invalidate_session(workspace, session, LoadReason::Compaction);
    }

    fn invalidate_session(&mut self, workspace: &Path, session: &str, reason: LoadReason) {
        let workspace = normalized_path(workspace);
        for key in
            self.entries.keys().filter(|key| key.workspace == workspace && key.session == session)
        {
            self.invalidations.insert(key.clone(), reason);
        }
    }
}

impl CacheKey {
    fn new(workspace: &Path, session: &str, filepath: &Path) -> Self {
        Self {
            workspace: normalized_path(workspace),
            session: session.to_string(),
            filepath: normalized_path(filepath),
        }
    }
}

impl CacheEntry {
    fn is_current(&self) -> bool {
        self.files.iter().all(FileState::is_current)
    }
}

impl FileState {
    fn read(filepath: &Path) -> Self {
        let metadata = fs::metadata(filepath).ok();
        Self {
            filepath: filepath.to_path_buf(),
            modified: metadata.as_ref().and_then(|metadata| metadata.modified().ok()),
            len: metadata.as_ref().map(fs::Metadata::len),
        }
    }

    fn is_current(&self) -> bool {
        Self::read(&self.filepath) == *self
    }
}

fn normalized_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn resolved_files(filepath: &Path, resolved: Option<&Resolved>) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    for source in resolved.into_iter().flat_map(|resolved| resolved.sources.iter()) {
        let source = PathBuf::from(&source.filepath);
        if seen.insert(source.clone()) {
            files.push(source);
        }
    }
    if files.is_empty() {
        files.push(normalized_path(filepath));
    }
    files
}
