//! 快照模块的原生平台实现。
//!
//! 该文件承载非 WASM 平台的实际快照逻辑，包括：
//! - 仓库初始化与清理
//! - 当前工作树跟踪与补丁生成
//! - 快照恢复与局部回退
//! - 两类差异计算接口

use super::common::{Error, LOGGER, PRUNE, Patch, extra, extra_from_output};
use super::{DiffStatus, FileDiff};
use crate::app::agent::global;
use crate::app::agent::scheduler;
use crate::app::agent::shell::git_std_command;
use git2::Repository;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::Arc;
use std::time::Duration;

/// 初始化快照清理任务。
///
/// 注册一个定时任务，定期清理过期的快照数据。任务每小时执行一次。
pub fn init(worktree: impl AsRef<Path>) {
    let worktree = worktree.as_ref().to_path_buf();
    let instance = worktree.to_string_lossy().to_string();

    let run: scheduler::RunFn = Arc::new(move || {
        let worktree = worktree.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || cleanup(&worktree))
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            Ok(())
        })
    });

    scheduler::register(scheduler::Task {
        id: "snapshot.cleanup".to_string(),
        interval: Duration::from_secs(60 * 60),
        run,
        scope: scheduler::Scope::Instance(instance),
    });
}

/// 清理过期的快照数据。
pub fn cleanup(worktree: impl AsRef<Path>) -> Result<(), Error> {
    let worktree = worktree.as_ref();

    let Some(project_id) = project_id(worktree) else {
        return Ok(());
    };

    let git = gitdir(&project_id);
    if !git.exists() {
        return Ok(());
    }

    let out =
        run_git(worktree, &git, ["gc", &format!("--prune={}", PRUNE)], None::<&[(&str, &str)]>)?;
    if !out.status.success() {
        LOGGER.warn(
            "cleanup failed",
            Some(extra_from_output(&out, [("prune", Value::String(PRUNE.to_string()))])),
        );
        return Ok(());
    }

    LOGGER.debug("cleanup", Some(extra([("prune", Value::String(PRUNE.to_string()))])));
    Ok(())
}

/// 跟踪当前工作目录状态。
pub fn track(worktree: impl AsRef<Path>) -> Result<Option<String>, Error> {
    let worktree = worktree.as_ref();

    let Some(project_id) = project_id(worktree) else {
        return Ok(None);
    };

    let git = gitdir(&project_id);
    let created = ensure_gitdir(&git)?;
    if created {
        init_repo(worktree, &git)?;
    }

    let _ = run_git(worktree, &git, ["add", "."], None::<&[(&str, &str)]>)?;
    let out = run_git(worktree, &git, ["write-tree"], None::<&[(&str, &str)]>)?;
    if !out.status.success() {
        return Ok(None);
    }

    let hash = String::from_utf8(out.stdout)?.trim().to_string();
    LOGGER.info(
        "tracking",
        Some(extra([
            ("hash", Value::String(hash.clone())),
            ("cwd", Value::String(worktree.to_string_lossy().to_string())),
            ("git", Value::String(git.to_string_lossy().to_string())),
        ])),
    );

    Ok(Some(hash))
}

/// 获取快照与当前状态之间的文件补丁信息。
pub fn patch(worktree: impl AsRef<Path>, hash: &str) -> Result<Patch, Error> {
    let worktree = worktree.as_ref();

    let Some(project_id) = project_id(worktree) else {
        return Ok(Patch { hash: hash.to_string(), files: Vec::new() });
    };

    let git = gitdir(&project_id);
    let _ = run_git(worktree, &git, ["add", "."], None::<&[(&str, &str)]>)?;
    let out = run_git(
        worktree,
        &git,
        [
            "-c",
            "core.autocrlf=false",
            "-c",
            "core.quotepath=false",
            "diff",
            "--no-ext-diff",
            "--name-only",
            hash,
            "--",
            ".",
        ],
        None::<&[(&str, &str)]>,
    )?;

    if !out.status.success() {
        LOGGER.warn(
            "failed to get diff",
            Some(extra([
                ("hash", Value::String(hash.to_string())),
                ("exit_code", Value::Number(out.status.code().unwrap_or(-1).into())),
            ])),
        );
        return Ok(Patch { hash: hash.to_string(), files: Vec::new() });
    }

    let text = String::from_utf8(out.stdout)?;
    let files = text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|rel| worktree.join(rel).to_string_lossy().to_string())
        .collect::<Vec<_>>();

    Ok(Patch { hash: hash.to_string(), files })
}

/// 恢复工作目录到指定快照状态。
pub fn restore(worktree: impl AsRef<Path>, snapshot: &str) -> Result<(), Error> {
    let worktree = worktree.as_ref();

    let Some(project_id) = project_id(worktree) else {
        return Ok(());
    };

    let git = gitdir(&project_id);
    LOGGER.info("restore", Some(extra([("commit", Value::String(snapshot.to_string()))])));

    let out = run_git(worktree, &git, ["read-tree", snapshot], None::<&[(&str, &str)]>)?;
    if !out.status.success() {
        LOGGER.error(
            "failed to restore snapshot",
            Some(extra_from_output(&out, [("snapshot", Value::String(snapshot.to_string()))])),
        );
        return Ok(());
    }

    let out = run_git(worktree, &git, ["checkout-index", "-a", "-f"], None::<&[(&str, &str)]>)?;
    if !out.status.success() {
        LOGGER.error(
            "failed to restore snapshot",
            Some(extra_from_output(&out, [("snapshot", Value::String(snapshot.to_string()))])),
        );
    }

    Ok(())
}

/// 回退指定的文件补丁。
pub fn revert(worktree: impl AsRef<Path>, patches: &[Patch]) -> Result<(), Error> {
    let worktree = worktree.as_ref();

    let Some(project_id) = project_id(worktree) else {
        return Ok(());
    };

    let git = gitdir(&project_id);
    let mut seen = HashSet::<String>::new();

    for item in patches {
        for file in &item.files {
            if !seen.insert(file.clone()) {
                continue;
            }

            LOGGER.info(
                "reverting",
                Some(extra([
                    ("file", Value::String(file.clone())),
                    ("hash", Value::String(item.hash.clone())),
                ])),
            );

            let rel = file_relative(worktree, file);
            let out = run_git(
                worktree,
                &git,
                ["checkout", &item.hash, "--", &rel],
                None::<&[(&str, &str)]>,
            )?;
            if out.status.success() {
                continue;
            }

            let check = run_git(
                worktree,
                &git,
                ["ls-tree", &item.hash, "--", &rel],
                None::<&[(&str, &str)]>,
            )?;
            let exists_in_tree =
                check.status.success() && !String::from_utf8(check.stdout)?.trim().is_empty();

            if exists_in_tree {
                LOGGER.info(
                    "file existed in snapshot but checkout failed, keeping",
                    Some(extra([("file", Value::String(file.clone()))])),
                );
            } else {
                LOGGER.info(
                    "file did not exist in snapshot, deleting",
                    Some(extra([("file", Value::String(file.clone()))])),
                );
                let _ = std::fs::remove_file(file);
            }
        }
    }

    Ok(())
}

/// 获取快照与当前状态之间的差异。
pub fn diff(worktree: impl AsRef<Path>, hash: &str) -> Result<String, Error> {
    let worktree = worktree.as_ref();

    let Some(project_id) = project_id(worktree) else {
        return Ok(String::new());
    };

    let git = gitdir(&project_id);
    let _ = run_git(worktree, &git, ["add", "."], None::<&[(&str, &str)]>)?;
    let out = run_git(
        worktree,
        &git,
        [
            "-c",
            "core.autocrlf=false",
            "-c",
            "core.quotepath=false",
            "diff",
            "--no-ext-diff",
            hash,
            "--",
            ".",
        ],
        None::<&[(&str, &str)]>,
    )?;

    if !out.status.success() {
        LOGGER.warn(
            "failed to get diff",
            Some(extra_from_output(&out, [("hash", Value::String(hash.to_string()))])),
        );
        return Ok(String::new());
    }

    Ok(String::from_utf8(out.stdout)?.trim().to_string())
}

/// 获取两个快照之间的完整差异信息。
pub fn diff_full(worktree: impl AsRef<Path>, from: &str, to: &str) -> Result<Vec<FileDiff>, Error> {
    let worktree = worktree.as_ref();

    let Some(project_id) = project_id(worktree) else {
        return Ok(Vec::new());
    };

    let git = gitdir(&project_id);
    let mut result = Vec::<FileDiff>::new();

    let statuses_out = run_git(
        worktree,
        &git,
        [
            "-c",
            "core.autocrlf=false",
            "-c",
            "core.quotepath=false",
            "diff",
            "--no-ext-diff",
            "--name-status",
            "--no-renames",
            from,
            to,
            "--",
            ".",
        ],
        None::<&[(&str, &str)]>,
    )?;

    let mut status = HashMap::<String, DiffStatus>::new();
    let statuses = String::from_utf8(statuses_out.stdout)?;
    for line in statuses.lines().map(|line| line.trim()).filter(|line| !line.is_empty()) {
        let mut parts = line.split('\t');
        let Some(code) = parts.next() else { continue };
        let Some(file) = parts.next() else { continue };

        let kind = if code.starts_with('A') {
            DiffStatus::Added
        } else if code.starts_with('D') {
            DiffStatus::Deleted
        } else {
            DiffStatus::Modified
        };
        status.insert(file.to_string(), kind);
    }

    let numstat_out = run_git(
        worktree,
        &git,
        [
            "-c",
            "core.autocrlf=false",
            "-c",
            "core.quotepath=false",
            "diff",
            "--no-ext-diff",
            "--no-renames",
            "--numstat",
            from,
            to,
            "--",
            ".",
        ],
        None::<&[(&str, &str)]>,
    )?;

    let numstat = String::from_utf8(numstat_out.stdout)?;
    for line in numstat.lines().map(|line| line.trim()).filter(|line| !line.is_empty()) {
        let parts = line.split('\t').collect::<Vec<_>>();
        if parts.len() < 3 {
            continue;
        }

        let additions_raw = parts[0];
        let deletions_raw = parts[1];
        let file = parts[2];
        let is_binary = additions_raw == "-" && deletions_raw == "-";
        let before = if is_binary { String::new() } else { show_file(worktree, &git, from, file)? };
        let after = if is_binary { String::new() } else { show_file(worktree, &git, to, file)? };
        let additions = if is_binary { 0 } else { additions_raw.parse::<i64>().unwrap_or(0) };
        let deletions = if is_binary { 0 } else { deletions_raw.parse::<i64>().unwrap_or(0) };

        result.push(FileDiff {
            file: file.to_string(),
            before,
            after,
            additions,
            deletions,
            status: status.get(file).cloned().or(Some(DiffStatus::Modified)),
        });
    }

    Ok(result)
}

fn ensure_gitdir(dir: &Path) -> Result<bool, Error> {
    if dir.exists() {
        return Ok(false);
    }
    std::fs::create_dir_all(dir)?;
    Ok(true)
}

fn init_repo(worktree: &Path, git: &Path) -> Result<(), Error> {
    let git_str = git.to_string_lossy().to_string();
    let work_str = worktree.to_string_lossy().to_string();
    let envs = [("GIT_DIR", git_str.as_str()), ("GIT_WORK_TREE", work_str.as_str())];

    let _ = run_git(worktree, git, ["init"], Some(envs.as_slice()))?;
    let _ = run_git(worktree, git, ["config", "core.autocrlf", "false"], None::<&[(&str, &str)]>)?;

    LOGGER.info("initialized", None);
    Ok(())
}

fn show_file(worktree: &Path, git: &Path, rev: &str, file: &str) -> Result<String, Error> {
    let spec = format!("{}:{}", rev, file);
    let out = run_git(
        worktree,
        git,
        ["-c", "core.autocrlf=false", "show", &spec],
        None::<&[(&str, &str)]>,
    )?;
    if !out.status.success() {
        return Ok(String::new());
    }
    Ok(String::from_utf8(out.stdout)?)
}

fn run_git<'a>(
    worktree: &Path,
    git: &Path,
    args: impl IntoIterator<Item = &'a str>,
    env: Option<&[(&str, &str)]>,
) -> Result<Output, Error> {
    let mut cmd = git_std_command();
    cmd.current_dir(worktree);
    cmd.arg("--git-dir").arg(git);
    cmd.arg("--work-tree").arg(worktree);

    for arg in args {
        cmd.arg(arg);
    }

    if let Some(env) = env {
        for (key, value) in env {
            cmd.env(key, value);
        }
    }

    Ok(cmd.output()?)
}

fn project_id(worktree: &Path) -> Option<String> {
    let Ok(repo) = Repository::discover(worktree) else {
        return None;
    };
    root_commit_id(&repo)
}

fn root_commit_id(repo: &Repository) -> Option<String> {
    let mut revwalk = repo.revwalk().ok()?;
    let Ok(mut refs) = repo.references() else {
        return None;
    };

    while let Some(reference) = refs.next() {
        let Ok(reference) = reference else { continue };
        let Some(oid) = reference.target() else { continue };
        let _ = revwalk.push(oid);
    }

    let mut roots = Vec::<String>::new();
    for oid in revwalk.flatten() {
        let Ok(commit) = repo.find_commit(oid) else { continue };
        if commit.parent_count() == 0 {
            roots.push(commit.id().to_string());
        }
    }

    roots.sort();
    roots.into_iter().next()
}

fn gitdir(project_id: &str) -> PathBuf {
    global::paths().data.join("snapshot").join(project_id)
}

fn file_relative(worktree: &Path, file: &str) -> String {
    let path = Path::new(file);
    if let Ok(rel) = path.strip_prefix(worktree) {
        return rel.to_string_lossy().to_string();
    }
    file.to_string()
}
#[cfg(test)]
#[path = "native_tests.rs"]
mod native_tests;
