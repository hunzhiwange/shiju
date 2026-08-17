//! 基于 git2 的结构化 Git 操作工具。

use super::traits::{Tool, ToolResult};
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, Utc};
use git2::{
    BranchType, DiffFormat, DiffOptions, Repository, Signature, Status, StatusOptions,
    build::CheckoutBuilder,
};
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct GitOperationsTool {
    security: Arc<SecurityPolicy>,
    workspace_dir: PathBuf,
}

impl GitOperationsTool {
    pub fn new(security: Arc<SecurityPolicy>, workspace_dir: PathBuf) -> Self {
        Self { security, workspace_dir }
    }

    fn requires_write_access(&self, operation: &str) -> bool {
        matches!(operation, "commit" | "add" | "checkout" | "stash" | "reset" | "revert")
    }

    fn is_read_only(&self, operation: &str) -> bool {
        matches!(operation, "status" | "diff" | "log" | "show" | "branch" | "rev-parse")
    }

    fn open_repository(&self) -> anyhow::Result<Repository> {
        Repository::discover(&self.workspace_dir)
            .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))
    }

    fn repository_signature(repo: &Repository) -> anyhow::Result<Signature<'static>> {
        let (name, email) = repo
            .signature()
            .map(|signature| {
                (
                    signature.name().unwrap_or("VibeWindow").to_owned(),
                    signature.email().unwrap_or("vibewindow@localhost").to_owned(),
                )
            })
            .unwrap_or_else(|_| ("VibeWindow".to_owned(), "vibewindow@localhost".to_owned()));
        Signature::now(&name, &email)
            .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))
    }

    fn validate_workspace_path(value: &str) -> anyhow::Result<&Path> {
        let path = Path::new(value);
        if value.is_empty()
            || value.contains('\0')
            || path.is_absolute()
            || path.components().any(|part| matches!(part, std::path::Component::ParentDir))
        {
            anyhow::bail!("Invalid repository path: {value}");
        }
        Ok(path)
    }

    fn validate_branch_name(value: &str) -> anyhow::Result<&str> {
        if value.trim().is_empty()
            || value != value.trim()
            || value.chars().any(char::is_whitespace)
        {
            anyhow::bail!("Invalid branch specification");
        }
        if value.contains('@')
            || value.contains('^')
            || value.contains('~')
            || value.contains(':')
            || value.contains('\0')
            || value.starts_with('-')
            || value.ends_with('.')
            || value.contains("..")
            || value.contains("//")
        {
            anyhow::bail!("Branch name contains invalid characters");
        }
        Ok(value)
    }

    async fn git_status(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let repo = self.open_repository()?;
        let mut options = StatusOptions::new();
        options.include_untracked(true).recurse_untracked_dirs(true).include_ignored(false);
        let statuses = repo
            .statuses(Some(&mut options))
            .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
        let mut staged = Vec::new();
        let mut unstaged = Vec::new();
        let mut untracked = Vec::new();
        for entry in statuses.iter() {
            let Ok(path) = entry.path() else { continue };
            let status = entry.status();
            if let Some(code) = index_status_code(status) {
                staged.push(json!({"path": path, "status": code}));
            }
            if status.contains(Status::WT_NEW) {
                untracked.push(path.to_owned());
            } else if let Some(code) = worktree_status_code(status) {
                unstaged.push(json!({"path": path, "status": code}));
            }
        }
        let branch = repo
            .head()
            .ok()
            .and_then(|head| head.shorthand().ok().map(str::to_owned))
            .unwrap_or_default();
        let clean = staged.is_empty() && unstaged.is_empty() && untracked.is_empty();
        Ok(json_result(
            json!({"branch": branch, "staged": staged, "unstaged": unstaged, "untracked": untracked, "clean": clean}),
        ))
    }

    async fn git_diff(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let files = args.get("files").and_then(|value| value.as_str()).unwrap_or(".");
        Self::validate_workspace_path(files)?;
        let cached = args.get("cached").and_then(|value| value.as_bool()).unwrap_or(false);
        let repo = self.open_repository()?;
        let mut options = DiffOptions::new();
        options.context_lines(3).pathspec(files);
        let diff = if cached {
            let tree = repo.head().ok().and_then(|head| head.peel_to_tree().ok());
            repo.diff_tree_to_index(tree.as_ref(), None, Some(&mut options))
        } else {
            repo.diff_index_to_workdir(None, Some(&mut options))
        }
        .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
        let mut patch = String::new();
        diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
            patch.push_str(&String::from_utf8_lossy(line.content()));
            true
        })
        .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
        let hunks = parse_diff_hunks(&patch);
        Ok(json_result(json!({"file_count": hunks.len(), "hunks": hunks})))
    }

    async fn git_log(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let limit =
            usize::try_from(args.get("limit").and_then(|value| value.as_u64()).unwrap_or(10))
                .unwrap_or(usize::MAX)
                .min(1000);
        let repo = self.open_repository()?;
        let mut walk =
            repo.revwalk().map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
        walk.push_head().map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
        let mut commits = Vec::new();
        for oid in walk.take(limit).flatten() {
            let commit = repo
                .find_commit(oid)
                .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
            let author = commit.author();
            commits.push(json!({
                "hash": oid.to_string(), "author": author.name().unwrap_or_default(),
                "email": author.email().unwrap_or_default(), "date": format_git_time(commit.time()),
                "message": commit.summary().unwrap_or_default()
            }));
        }
        Ok(json_result(json!({"commits": commits})))
    }

    async fn git_branch(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let repo = self.open_repository()?;
        let current = repo
            .head()
            .ok()
            .and_then(|head| head.shorthand().ok().map(str::to_owned))
            .unwrap_or_default();
        let mut branches = Vec::new();
        for branch in repo
            .branches(Some(BranchType::Local))
            .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?
        {
            let (branch, _) =
                branch.map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
            let name = branch
                .name()
                .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?
                .unwrap_or_default()
                .to_owned();
            branches.push(json!({"name": name, "current": name == current}));
        }
        Ok(json_result(json!({"current": current, "branches": branches})))
    }

    fn truncate_commit_message(message: &str) -> String {
        if message.chars().count() > 2000 {
            format!("{}...", message.chars().take(1997).collect::<String>())
        } else {
            message.to_owned()
        }
    }

    async fn git_commit(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let message = args
            .get("message")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'message' parameter"))?;
        let message = message
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if message.is_empty() {
            anyhow::bail!("Commit message cannot be empty");
        }
        let message = Self::truncate_commit_message(&message);
        let result = (|| -> anyhow::Result<()> {
            let repo = self.open_repository()?;
            let signature = Self::repository_signature(&repo)?;
            let mut index =
                repo.index().map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
            let tree_id = index
                .write_tree()
                .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
            let tree = repo
                .find_tree(tree_id)
                .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
            let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
            let parents = parent.iter().collect::<Vec<_>>();
            repo.commit(Some("HEAD"), &signature, &signature, &message, &tree, &parents)
                .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
            Ok(())
        })();
        Ok(match result {
            Ok(()) => {
                ToolResult { success: true, output: format!("Committed: {message}"), error: None }
            }
            Err(error) => failure("Commit", error),
        })
    }

    async fn git_add(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let paths = args
            .get("paths")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'paths' parameter"))?;
        let path = Self::validate_workspace_path(paths)?;
        let result = (|| -> anyhow::Result<()> {
            let repo = self.open_repository()?;
            let mut index =
                repo.index().map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
            index
                .add_path(path)
                .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
            index.write().map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
            Ok(())
        })();
        Ok(match result {
            Ok(()) => ToolResult { success: true, output: format!("Staged: {paths}"), error: None },
            Err(error) => failure("Add", error),
        })
    }

    async fn git_checkout(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let branch = args
            .get("branch")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'branch' parameter"))?;
        let branch = Self::validate_branch_name(branch)?;
        let result = (|| -> anyhow::Result<()> {
            let repo = self.open_repository()?;
            repo.find_branch(branch, BranchType::Local)
                .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
            repo.set_head(&format!("refs/heads/{branch}"))
                .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
            let mut checkout = CheckoutBuilder::new();
            checkout.safe();
            repo.checkout_head(Some(&mut checkout))
                .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
            Ok(())
        })();
        Ok(match result {
            Ok(()) => ToolResult {
                success: true,
                output: format!("Switched to branch: {branch}"),
                error: None,
            },
            Err(error) => failure("Checkout", error),
        })
    }

    async fn git_stash(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let action = args.get("action").and_then(|value| value.as_str()).unwrap_or("push");
        let result = (|| -> anyhow::Result<String> {
            let mut repo = self.open_repository()?;
            match action {
                "push" | "save" => {
                    let signature = Self::repository_signature(&repo)?;
                    repo.stash_save(&signature, "auto-stash", None)
                        .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
                    Ok(String::new())
                }
                "pop" => {
                    repo.stash_pop(0, None)
                        .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
                    Ok(String::new())
                }
                "list" => {
                    let mut entries = Vec::new();
                    repo.stash_foreach(|index, name, oid| {
                        entries.push(format!("stash@{{{index}}}: {name} {oid}"));
                        true
                    })
                    .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
                    Ok(entries.join("\n"))
                }
                "drop" => {
                    let index = usize::try_from(
                        args.get("index").and_then(|value| value.as_u64()).unwrap_or(0),
                    )
                    .map_err(|_| anyhow::anyhow!("stash index too large"))?;
                    repo.stash_drop(index)
                        .map_err(|error| anyhow::anyhow!("Git operation failed: {error}"))?;
                    Ok(String::new())
                }
                _ => anyhow::bail!("Unknown stash action: {action}. Use: push, pop, list, drop"),
            }
        })();
        Ok(match result {
            Ok(output) => ToolResult { success: true, output, error: None },
            Err(error) => failure(&format!("Stash {action}"), error),
        })
    }
}

fn index_status_code(status: Status) -> Option<char> {
    status_code(
        status,
        Status::INDEX_NEW,
        Status::INDEX_MODIFIED,
        Status::INDEX_DELETED,
        Status::INDEX_RENAMED,
        Status::INDEX_TYPECHANGE,
    )
}
fn worktree_status_code(status: Status) -> Option<char> {
    status_code(
        status,
        Status::WT_NEW,
        Status::WT_MODIFIED,
        Status::WT_DELETED,
        Status::WT_RENAMED,
        Status::WT_TYPECHANGE,
    )
}
fn status_code(
    status: Status,
    new: Status,
    modified: Status,
    deleted: Status,
    renamed: Status,
    typechange: Status,
) -> Option<char> {
    if status.contains(Status::CONFLICTED) {
        Some('U')
    } else if status.contains(new) {
        Some('A')
    } else if status.contains(modified) {
        Some('M')
    } else if status.contains(deleted) {
        Some('D')
    } else if status.contains(renamed) {
        Some('R')
    } else if status.contains(typechange) {
        Some('T')
    } else {
        None
    }
}
fn format_git_time(time: git2::Time) -> String {
    let offset = FixedOffset::east_opt(time.offset_minutes() * 60)
        .unwrap_or_else(|| FixedOffset::east_opt(0).unwrap());
    DateTime::<Utc>::from_timestamp(time.seconds(), 0)
        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
        .with_timezone(&offset)
        .format("%Y-%m-%d %H:%M:%S %z")
        .to_string()
}
fn parse_diff_hunks(patch: &str) -> Vec<serde_json::Value> {
    let mut hunks = Vec::new();
    let mut file = String::new();
    let mut current = serde_json::Map::new();
    let mut lines = Vec::new();
    let finish = |hunks: &mut Vec<serde_json::Value>,
                  current: &mut serde_json::Map<String, serde_json::Value>,
                  lines: &mut Vec<serde_json::Value>| {
        if !lines.is_empty() {
            current.insert("lines".into(), json!(std::mem::take(lines)));
            if !current.is_empty() {
                hunks.push(serde_json::Value::Object(std::mem::take(current)));
            }
        }
    };
    for line in patch.lines() {
        if let Some(path) = line
            .strip_prefix("diff --git ")
            .and_then(|value| value.split_whitespace().nth(1))
            .map(|value| value.trim_start_matches("b/"))
        {
            finish(&mut hunks, &mut current, &mut lines);
            file = path.to_owned();
            current.insert("file".into(), json!(file));
        } else if line.starts_with("@@ ") {
            finish(&mut hunks, &mut current, &mut lines);
            current.insert("file".into(), json!(file));
            current.insert("header".into(), json!(line));
        } else if !line.is_empty() {
            lines.push(json!({"text": line, "type": if line.starts_with('+') { "add" } else if line.starts_with('-') { "delete" } else { "context" }}));
        }
    }
    finish(&mut hunks, &mut current, &mut lines);
    hunks
}
fn json_result(value: serde_json::Value) -> ToolResult {
    ToolResult {
        success: true,
        output: serde_json::to_string_pretty(&value).unwrap_or_default(),
        error: None,
    }
}
fn failure(operation: &str, error: anyhow::Error) -> ToolResult {
    ToolResult {
        success: false,
        output: String::new(),
        error: Some(format!("{operation} failed: {error}")),
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for GitOperationsTool {
    fn name(&self) -> &str {
        "git_operations"
    }
    fn description(&self) -> &str {
        "执行结构化 Git 操作（status、diff、log、branch、commit、add、checkout、stash）。提供解析后的 JSON 输出，并与安全策略集成实现自主控制。"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{"operation":{"type":"string","enum":["status","diff","log","branch","commit","add","checkout","stash"],"description":"要执行的 Git 操作"},"message":{"type":"string","description":"提交消息（用于 'commit' 操作）"},"paths":{"type":"string","description":"要暂存的文件路径（用于 'add' 操作）"},"branch":{"type":"string","description":"分支名称（用于 'checkout' 操作）"},"files":{"type":"string","description":"要对比的文件或路径（用于 'diff' 操作，默认：'.'）"},"cached":{"type":"boolean","description":"显示暂存差异（用于 'diff' 操作）"},"limit":{"type":"integer","description":"日志条目数量（用于 'log' 操作，默认：10）"},"action":{"type":"string","enum":["push","pop","list","drop"],"description":"Stash 操作（用于 'stash' 操作）"},"index":{"type":"integer","description":"Stash 索引（用于带 'drop' 操作的 'stash'）"}},"required":["operation"]})
    }
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let operation = match args.get("operation").and_then(|value| value.as_str()) {
            Some(operation) => operation,
            None => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Missing 'operation' parameter".into()),
                });
            }
        };
        if Repository::discover(&self.workspace_dir).is_err() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Not in a git repository".into()),
            });
        }
        if self.requires_write_access(operation) {
            if !self.security.can_act() {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(
                        "Action blocked: git write operations require higher autonomy level".into(),
                    ),
                });
            }
            if matches!(self.security.autonomy, AutonomyLevel::ReadOnly) {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Action blocked: read-only mode".into()),
                });
            }
        }
        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: rate limit exceeded".into()),
            });
        }
        match operation {
            "status" => self.git_status(args).await,
            "diff" => self.git_diff(args).await,
            "log" => self.git_log(args).await,
            "branch" => self.git_branch(args).await,
            "commit" => self.git_commit(args).await,
            "add" => self.git_add(args).await,
            "checkout" => self.git_checkout(args).await,
            "stash" => self.git_stash(args).await,
            _ => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown operation: {operation}")),
            }),
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
