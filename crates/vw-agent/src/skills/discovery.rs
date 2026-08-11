//! 技能目录发现与来源合并。
//!
//! 每次发现都从磁盘重建目录列表，避免将启动时的结果缓存为静态技能集。

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 返回来源目录下所有含技能清单的目录，按路径确定性排序。
///
/// 符号链接不跟随；加载器仍会对每个结果执行安全审计。
pub(crate) fn discover_skill_directories(skills_dir: &Path) -> Vec<PathBuf> {
    if !skills_dir.is_dir() {
        return Vec::new();
    }

    let mut seen = HashSet::new();
    let mut directories = WalkDir::new(skills_dir)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| matches!(entry.file_name().to_str(), Some("SKILL.md" | "SKILL.toml")))
        .filter_map(|entry| entry.path().parent().map(Path::to_path_buf))
        .filter(|path| {
            let realpath = path.canonicalize().unwrap_or_else(|_| path.clone());
            seen.insert(realpath)
        })
        .collect::<Vec<_>>();

    directories.sort();
    directories
}
