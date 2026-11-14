//! 基于 git2 的只读仓库发现。
//!
//! 本模块用 libgit2 替代外部 `git` 可执行文件，提供项目发现与分支读取所需的
//! 最小只读能力：仓库发现、工作树根、公共 git 目录、根提交 ID 与当前分支。
//! 不包含 `init`、提交、合并或远程写操作。

use git2::{Repository, Sort};
use std::path::{Path, PathBuf};

/// 已发现的 Git 仓库只读摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredRepository {
    /// 当前检出工作树根目录（等价于 `rev-parse --show-toplevel`）。
    pub workdir: PathBuf,
    /// 公共 git 目录（等价于 `rev-parse --git-common-dir` 的规范化结果）。
    pub common_dir: PathBuf,
    /// 公共 git 目录的父目录，用作项目 worktree 身份路径。
    pub common_root: PathBuf,
    /// 按字典序排序的根提交 ID 列表（无父提交）；空仓库为空。
    pub root_commit_ids: Vec<String>,
}

/// 从任意路径向上发现 Git 仓库。
///
/// 对普通仓库、空仓库与 linked worktree 均返回摘要；裸仓库或无法解析工作树
/// 时返回 `None`。本函数不会调用 `Repository::init`。
pub fn discover(path: &Path) -> Option<DiscoveredRepository> {
    let repo = Repository::discover(path).ok()?;
    let workdir = repo.workdir()?.to_path_buf();
    let workdir = canonicalize(&workdir);
    let common_dir = canonicalize(repo.commondir());
    let common_root = common_dir
        .parent()
        .map(Path::to_path_buf)
        .filter(|parent| parent.as_os_str() != ".")
        .unwrap_or_else(|| workdir.clone());
    let root_commit_ids = root_commit_ids(&repo);
    Some(DiscoveredRepository { workdir, common_dir, common_root, root_commit_ids })
}

/// 读取当前分支简称。
///
/// 行为对齐 `git rev-parse --abbrev-ref HEAD`：普通分支返回分支名，detached
/// HEAD 返回 `"HEAD"`，尚未产生提交的符号 HEAD 返回目标分支名。
pub fn current_branch(path: &Path) -> Option<String> {
    let repo = Repository::discover(path).ok()?;
    match repo.head() {
        Ok(head) => {
            if head.is_branch() {
                head.shorthand().ok().map(str::to_string).filter(|name| !name.is_empty())
            } else {
                Some("HEAD".to_string())
            }
        }
        Err(_) => repo.find_reference("HEAD").ok().and_then(|reference| {
            reference
                .symbolic_target()
                .ok()
                .flatten()
                .and_then(|target| target.strip_prefix("refs/heads/"))
                .map(str::to_string)
                .filter(|name| !name.is_empty())
        }),
    }
}

fn root_commit_ids(repo: &Repository) -> Vec<String> {
    let Ok(mut revwalk) = repo.revwalk() else {
        return Vec::new();
    };
    if revwalk.set_sorting(Sort::NONE).is_err() {
        return Vec::new();
    }
    // 对齐 `git rev-list --max-parents=0 --all`：遍历全部引用可达提交。
    if revwalk.push_glob("refs/*").is_err() {
        return Vec::new();
    }

    let mut roots = Vec::new();
    for oid in revwalk.flatten() {
        let Ok(commit) = repo.find_commit(oid) else {
            continue;
        };
        if commit.parent_count() == 0 {
            roots.push(oid.to_string());
        }
    }
    roots.sort();
    roots.dedup();
    roots
}

fn canonicalize(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
#[path = "git_repository_tests.rs"]
mod git_repository_tests;
