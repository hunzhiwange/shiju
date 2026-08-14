//! Git 工作区文件状态（git2）。
//!
//! 本模块通过 libgit2 读取相对 HEAD 的变更与未跟踪文件，不启动系统 `git`。
//! 仍依赖 Git CLI 的范围外能力：`/v1/git/command`、普通 worktree fetch/submodule、
//! skill clone/pull、snapshot 等。

use super::{Info, Status};
use git2::{Delta, DiffFormat, DiffOptions, Repository, StatusOptions, StatusShow};
use std::collections::HashMap;
use std::path::Path;

/// 获取 Git 工作区的文件状态。
///
/// 行为对齐原 CLI 路径：相对 HEAD 的已跟踪变更（含暂存新增）记为
/// [`Status::Modified`]；未跟踪文件记为 [`Status::Added`] 并以文件行数计
/// `added`；删除路径额外附带 [`Status::Deleted`] 条目。非 Git / 缺失路径
/// 返回空列表。
pub fn status_git(worktree: impl AsRef<Path>) -> Vec<Info> {
    let worktree = worktree.as_ref();
    let Ok(repo) = Repository::open(worktree) else {
        return Vec::new();
    };

    let Some(head_tree) = repo.head().ok().and_then(|head| head.peel_to_tree().ok()) else {
        return untracked_only(&repo, worktree);
    };

    let mut opts = DiffOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true)
        .show_binary(true);

    let Ok(diff) = repo.diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut opts)) else {
        return Vec::new();
    };

    let mut line_counts = HashMap::<String, (i64, i64)>::new();
    let _ = diff.print(DiffFormat::Patch, |delta, _hunk, line| {
        if is_untracked_delta(delta.status()) {
            return true;
        }
        let Some(path) = delta_path(&delta) else {
            return true;
        };
        let entry = line_counts.entry(path).or_insert((0, 0));
        match line.origin() {
            '+' => entry.0 += 1,
            '-' => entry.1 += 1,
            _ => {}
        }
        true
    });

    let mut changed = Vec::new();
    for delta in diff.deltas() {
        let status = delta.status();
        let Some(path) = delta_path(&delta) else {
            continue;
        };
        match status {
            Delta::Untracked => {
                let full = worktree.join(&path);
                let lines = std::fs::read_to_string(&full)
                    .map(|content| content.lines().count() as i64)
                    .unwrap_or(0);
                changed.push(Info { path, added: lines, removed: 0, status: Status::Added });
            }
            Delta::Deleted => {
                let (added, removed) = line_counts.get(&path).copied().unwrap_or((0, 0));
                // 对齐 `git diff --numstat HEAD`：删除文件先以 Modified 计数出现。
                changed.push(Info { path: path.clone(), added, removed, status: Status::Modified });
                changed.push(Info { path, added: 0, removed: 0, status: Status::Deleted });
            }
            Delta::Added | Delta::Modified | Delta::Renamed | Delta::Copied | Delta::Typechange => {
                let (added, removed) = line_counts.get(&path).copied().unwrap_or((0, 0));
                changed.push(Info { path, added, removed, status: Status::Modified });
            }
            _ => {}
        }
    }

    changed
}

fn untracked_only(repo: &Repository, worktree: &Path) -> Vec<Info> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .show(StatusShow::IndexAndWorkdir);

    let Ok(statuses) = repo.statuses(Some(&mut opts)) else {
        return Vec::new();
    };

    let mut changed = Vec::new();
    for entry in statuses.iter() {
        let status = entry.status();
        if !status.contains(git2::Status::WT_NEW) && !status.contains(git2::Status::INDEX_NEW) {
            continue;
        }
        let Ok(path) = entry.path() else {
            continue;
        };
        // 尚未产生 HEAD 时，暂存新增与未跟踪均按 Added 暴露（对齐 ls-files --others
        // 在无 HEAD 时仍可列出工作区新文件的行为）。
        let full = worktree.join(path);
        let lines = std::fs::read_to_string(&full)
            .map(|content| content.lines().count() as i64)
            .unwrap_or(0);
        changed.push(Info {
            path: path.to_string(),
            added: lines,
            removed: 0,
            status: Status::Added,
        });
    }
    changed
}

fn is_untracked_delta(status: Delta) -> bool {
    matches!(status, Delta::Untracked)
}

fn delta_path(delta: &git2::DiffDelta<'_>) -> Option<String> {
    delta
        .new_file()
        .path()
        .or_else(|| delta.old_file().path())
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}
