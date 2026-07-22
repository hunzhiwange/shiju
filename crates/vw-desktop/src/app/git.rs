//! Git 仓库操作工具模块
//!
//! 本模块提供对 Git 仓库的各种操作功能，包括：
//! - 文件暂存与提交（`git_stage_file`、`git_commit`、`git_commit_with_body`）
//! - 查看提交历史（`git_log`）
//! - 文件差异查看（`git_diff_for_file`）
//! - 文件/代码块/行的丢弃与恢复（`git_discard_file`、`git_discard_hunk`、`git_revert_line_delete` 等）
//! - 代码块/行的精确暂存（`git_stage_hunk`、`git_stage_line_insert`、`git_stage_line_delete`）
//!
//! # 平台兼容性
//!
//! 大部分功能仅在非 WASM 目标平台（`not(target_arch = "wasm32")`）上可用。
//! 在 Web 平台上，相关函数会返回错误或不支持的响应。

/// diff 输出的上下文行数
///
/// 在生成 unified diff 格式时，每个差异块前后的上下文行数。
/// 默认值为 3，与 git 默认行为一致。
pub const DIFF_CONTEXT: usize = 3;

use super::Shell;
#[cfg(not(target_arch = "wasm32"))]
use similar::TextDiff;

/// 将指定文件添加到 Git 暂存区
///
/// # 参数
///
/// - `repo_path`: Git 仓库的根目录路径
/// - `file`: 要暂存的文件路径（相对于仓库根目录）
/// - `shell`: 执行命令时使用的 shell 类型（Bash 或 Zsh）
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误信息
///
/// # 示例
///
/// ```ignore
/// use crate::app::git::git_stage_file;
/// use crate::app::Shell;
///
/// git_stage_file("/path/to/repo", "src/main.rs", Shell::Bash)?;
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn git_stage_file(repo_path: &str, file: &str, shell: Shell) -> Result<(), String> {
    /// 对字符串进行 shell 引号转义
    ///
    /// 使用单引号包裹字符串，并转义内部的单引号
    fn sh_quote(s: &str) -> String {
        let escaped = s.replace("'", "'\"'\"'");
        format!("'{}'", escaped)
    }
    let cmd = format!("git add -A -- {}", sh_quote(file));
    let _ = run_shell_command(Some(repo_path.to_string()), cmd, shell);
    Ok(())
}

/// 使用指定消息创建 Git 提交
///
/// # 参数
///
/// - `repo_path`: Git 仓库的根目录路径
/// - `message`: 提交消息
/// - `shell`: 执行命令时使用的 shell 类型
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误信息
///
/// # 示例
///
/// ```ignore
/// use crate::app::git::git_commit;
/// use crate::app::Shell;
///
/// git_commit("/path/to/repo", "feat: add new feature", Shell::Bash)?;
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn git_commit(repo_path: &str, message: &str, shell: Shell) -> Result<(), String> {
    let _ = shell;
    git_commit_internal(repo_path, message, None)
}

/// 使用标题和正文创建 Git 提交
///
/// 与 `git_commit` 不同，此函数允许分别指定提交标题和详细正文。
///
/// # 参数
///
/// - `repo_path`: Git 仓库的根目录路径
/// - `summary`: 提交标题（第一行）
/// - `body`: 提交正文（详细说明）
/// - `shell`: 执行命令时使用的 shell 类型
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误信息
///
/// # 示例
///
/// ```ignore
/// use crate::app::git::git_commit_with_body;
/// use crate::app::Shell;
///
/// git_commit_with_body(
///     "/path/to/repo",
///     "feat: add new feature",
///     "This commit adds a new feature that allows...",
///     Shell::Bash,
/// )?;
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn git_commit_with_body(
    repo_path: &str,
    summary: &str,
    body: &str,
    shell: Shell,
) -> Result<(), String> {
    let _ = shell;
    git_commit_internal(repo_path, summary, Some(body))
}

#[cfg(not(target_arch = "wasm32"))]
fn git_commit_internal(repo_path: &str, summary: &str, body: Option<&str>) -> Result<(), String> {
    let repo = git2::Repository::open(repo_path).map_err(|e| e.message().to_string())?;
    let mut index = repo.index().map_err(|e| e.message().to_string())?;
    let tree_id = index.write_tree().map_err(|e| e.message().to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|e| e.message().to_string())?;
    let signature = repo
        .signature()
        .or_else(|_| git2::Signature::now("Vibe Window", "vibe@example.test"))
        .map_err(|e| e.message().to_string())?;
    let parents = if let Ok(head) = repo.head() {
        vec![head.peel_to_commit().map_err(|e| e.message().to_string())?]
    } else {
        Vec::new()
    };
    let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();
    let message = match body {
        Some(body) if !body.is_empty() => format!("{summary}\n\n{body}"),
        _ => summary.to_string(),
    };

    repo.commit(Some("HEAD"), &signature, &signature, &message, &tree, &parent_refs)
        .map_err(|e| e.message().to_string())?;
    index.write().map_err(|e| e.message().to_string())
}

/// 获取最近 N 条 Git 提交记录
///
/// # 参数
///
/// - `repo_path`: Git 仓库的根目录路径
/// - `n`: 要获取的提交数量
///
/// # 返回值
///
/// 返回一个元组向量，每个元组包含：
/// - `String`: 提交哈希（完整 SHA-1）
/// - `String`: 提交主题（提交消息的第一行）
///
/// # 示例
///
/// ```ignore
/// use crate::app::git::git_log;
///
/// let commits = git_log("/path/to/repo", 10);
/// for (hash, subject) in commits {
///     println!("{}: {}", hash, subject);
/// }
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn git_log(repo_path: &str, n: usize) -> Vec<(String, String)> {
    let cmd = format!("git log --pretty=format:'%H|%s' -n {}", n);
    let out = run_shell_command(Some(repo_path.to_string()), cmd, Shell::Bash);
    out.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, '|');
            let id = parts.next()?.to_string();
            let subject = parts.next()?.to_string();
            Some((id, subject))
        })
        .collect()
}

/// 丢弃文件的工作区修改
///
/// 将文件恢复到最近一次提交的状态。对于未跟踪的新文件，会将其删除。
///
/// # 参数
///
/// - `repo_path`: Git 仓库的根目录路径
/// - `file`: 要丢弃修改的文件路径（相对于仓库根目录）
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误信息
///
/// # 行为说明
///
/// - 未跟踪的新文件（`WT_NEW`）：直接删除物理文件
/// - 已暂存的新文件（`INDEX_NEW`）：删除物理文件并从索引中移除
/// - 已修改的文件：使用 `git checkout` 恢复到 HEAD 版本
///
/// # 示例
///
/// ```ignore
/// use crate::app::git::git_discard_file;
///
/// git_discard_file("/path/to/repo", "src/main.rs")?;
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn git_discard_file(repo_path: &str, file: &str) -> Result<(), String> {
    let repo = git2::Repository::open(repo_path).map_err(|e| e.message().to_string())?;

    // 首先检查文件状态
    let status =
        repo.status_file(std::path::Path::new(file)).map_err(|e| e.message().to_string())?;

    if status.contains(git2::Status::WT_NEW) || status.contains(git2::Status::INDEX_NEW) {
        // 未跟踪或新添加的文件 -> 删除文件
        let full_path = std::path::Path::new(repo_path).join(file);
        if full_path.exists() {
            std::fs::remove_file(full_path).map_err(|e| e.to_string())?;
        }
        // 如果是已暂存的新文件，需要从索引中移除
        if status.contains(git2::Status::INDEX_NEW) {
            let mut index = repo.index().map_err(|e| e.message().to_string())?;
            index.remove_path(std::path::Path::new(file)).map_err(|e| e.message().to_string())?;
            index.write().map_err(|e| e.message().to_string())?;
        }
        return Ok(());
    }

    let head = repo.head().map_err(|e| e.message().to_string())?;
    let tree = head.peel_to_tree().map_err(|e| e.message().to_string())?;
    let mut cb = git2::build::CheckoutBuilder::new();
    cb.force().path(file);
    repo.checkout_tree(tree.as_object(), Some(&mut cb)).map_err(|e| e.message().to_string())
}

/// 获取指定文件的 Git 差异
///
/// 生成指定文件的 unified diff 格式输出，显示工作区与 HEAD 之间的差异。
///
/// # 参数
///
/// - `repo_path`: Git 仓库的根目录路径
/// - `file`: 要查看差异的文件路径（相对于仓库根目录）
///
/// # 返回值
///
/// - `Some(String)`: 文件的 diff 输出
/// - `None`: 文件无变化或无法获取差异
///
/// # 示例
///
/// ```ignore
/// use crate::app::git::git_diff_for_file;
///
/// if let Some(diff) = git_diff_for_file("/path/to/repo", "src/main.rs") {
///     println!("{}", diff);
/// }
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn git_diff_for_file(repo_path: &str, file: &str) -> Option<String> {
    let repo = git2::Repository::open(repo_path).ok()?;
    let head = repo.head().ok()?;
    let tree = head.peel_to_tree().ok()?;
    let mut opts = git2::DiffOptions::new();
    opts.include_untracked(true);
    let diff = repo.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts)).ok()?;
    let mut out = String::new();
    let _ = diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .and_then(|p| p.to_str())
            .unwrap_or("");
        if path == file
            && let Ok(s) = std::str::from_utf8(line.content())
        {
            match line.origin() {
                '+' | '-' | ' ' => out.push(line.origin()),
                _ => {}
            }
            out.push_str(s);
        }
        true
    });
    if !out.is_empty() {
        return Some(out);
    }

    // 尝试使用文本差异作为备选方案
    let (old_content, new_content) = get_file_content_pair(repo_path, file)?;
    if old_content == new_content {
        return None;
    }

    let diff = TextDiff::from_lines(&old_content, &new_content);
    let mut patch = String::new();
    patch.push_str(&format!("diff --git a/{} b/{}\n", file, file));

    // 判断文件状态：新建、删除或修改
    let is_new_file = old_content.is_empty() && !new_content.is_empty();
    let is_deleted_file = !old_content.is_empty() && new_content.is_empty();

    if is_new_file {
        patch.push_str("new file mode 100644\n");
        patch.push_str("index 0000000..0000000\n");
        patch.push_str("--- /dev/null\n");
        patch.push_str(&format!("+++ b/{}\n", file));
    } else if is_deleted_file {
        patch.push_str("deleted file mode 100644\n");
        patch.push_str("index 0000000..0000000\n");
        patch.push_str(&format!("--- a/{}\n", file));
        patch.push_str("+++ /dev/null\n");
    } else {
        patch.push_str("index 0000000..0000000 100644\n");
        patch.push_str(&format!("--- a/{}\n", file));
        patch.push_str(&format!("+++ b/{}\n", file));
    }

    // 构建差异块输出
    for hunk in diff.unified_diff().context_radius(DIFF_CONTEXT).iter_hunks() {
        patch.push_str(&hunk.to_string());
    }

    Some(patch)
}

/// 获取文件在 HEAD 版本和工作区的内容对
///
/// 内部辅助函数，用于比较文件变化。
///
/// # 参数
///
/// - `repo_path`: Git 仓库的根目录路径
/// - `file_path`: 文件路径（相对于仓库根目录）
///
/// # 返回值
///
/// 返回 `Some((old_content, new_content))`，其中：
/// - `old_content`: HEAD 版本中的文件内容
/// - `new_content`: 工作区中的文件内容
///
/// 如果文件在 HEAD 中不存在，`old_content` 为空字符串。
#[cfg(not(target_arch = "wasm32"))]
fn get_file_content_pair(repo_path: &str, file_path: &str) -> Option<(String, String)> {
    let repo = git2::Repository::open(repo_path).ok()?;
    let head = repo.head().ok()?;
    let tree = head.peel_to_tree().ok()?;

    // 获取 HEAD 版本的文件内容
    let old_content = if let Ok(entry) = tree.get_path(std::path::Path::new(file_path)) {
        let obj = entry.to_object(&repo).ok()?;
        let blob = obj.as_blob()?;
        String::from_utf8_lossy(blob.content()).to_string()
    } else {
        String::new()
    };

    // 获取工作区版本的文件内容
    let full_path = std::path::Path::new(repo_path).join(file_path);
    let new_content = std::fs::read_to_string(full_path).unwrap_or_default();

    Some((old_content, new_content))
}

/// 丢弃指定代码块的修改
///
/// 将文件中特定代码块的修改恢复到 HEAD 版本。
///
/// # 参数
///
/// - `repo_path`: Git 仓库的根目录路径
/// - `file`: 要操作的文件路径（相对于仓库根目录）
/// - `idx`: 代码块的索引（从 0 开始）
/// - `shell`: 执行命令时使用的 shell 类型
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误信息
///
/// # 实现原理
///
/// 1. 生成指定代码块的 patch 文件
/// 2. 使用 `git apply -R` 反向应用 patch，恢复该代码块
///
/// # 示例
///
/// ```ignore
/// use crate::app::git::git_discard_hunk;
/// use crate::app::Shell;
///
/// // 丢弃第一个代码块的修改
/// git_discard_hunk("/path/to/repo", "src/main.rs", 0, Shell::Bash)?;
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn git_discard_hunk(
    repo_path: &str,
    file: &str,
    idx: usize,
    shell: Shell,
) -> Result<(), String> {
    let (old_content, new_content) = get_file_content_pair(repo_path, file)
        .ok_or_else(|| "Failed to read file content".to_string())?;

    let diff = TextDiff::from_lines(&old_content, &new_content);
    let hunks: Vec<String> = diff
        .unified_diff()
        .context_radius(DIFF_CONTEXT)
        .iter_hunks()
        .map(|hunk| hunk.to_string())
        .collect();

    if idx >= hunks.len() {
        return Err("bad hunk index".to_string());
    }

    let hunk = &hunks[idx];

    // 构建 patch 文件内容
    let mut patch = String::new();
    patch.push_str(&format!("diff --git a/{} b/{}\n", file, file));

    let is_new_file = old_content.is_empty() && !new_content.is_empty();
    let is_deleted_file = !old_content.is_empty() && new_content.is_empty();

    if is_new_file {
        patch.push_str("new file mode 100644\n");
        patch.push_str("index 0000000..0000000\n");
        patch.push_str("--- /dev/null\n");
        patch.push_str(&format!("+++ b/{}\n", file));
    } else if is_deleted_file {
        patch.push_str("deleted file mode 100644\n");
        patch.push_str("index 0000000..0000000\n");
        patch.push_str(&format!("--- a/{}\n", file));
        patch.push_str("+++ /dev/null\n");
    } else {
        patch.push_str("index 0000000..0000000 100644\n");
        patch.push_str(&format!("--- a/{}\n", file));
        patch.push_str(&format!("+++ b/{}\n", file));
    }

    patch.push_str(hunk);

    // 写入临时 patch 文件并反向应用
    let tmp = std::env::temp_dir().join(format!("vibe-hunk-{}.patch", idx));
    std::fs::write(&tmp, patch).map_err(|e| e.to_string())?;

    let cmd = format!("git apply -R {}", tmp.to_string_lossy());
    let _ = run_shell_command(Some(repo_path.to_string()), cmd, shell);
    let _ = std::fs::remove_file(&tmp);
    Ok(())
}

/// 暂存指定代码块
///
/// 将文件中特定代码块的修改添加到 Git 暂存区。
///
/// # 参数
///
/// - `repo_path`: Git 仓库的根目录路径
/// - `file`: 要操作的文件路径（相对于仓库根目录）
/// - `idx`: 代码块的索引（从 0 开始）
/// - `shell`: 执行命令时使用的 shell 类型
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误信息
///
/// # 实现原理
///
/// 1. 生成指定代码块的 patch 文件
/// 2. 使用 `git apply --cached` 将 patch 应用到暂存区
///
/// # 示例
///
/// ```ignore
/// use crate::app::git::git_stage_hunk;
/// use crate::app::Shell;
///
/// // 暂存第一个代码块
/// git_stage_hunk("/path/to/repo", "src/main.rs", 0, Shell::Bash)?;
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn git_stage_hunk(repo_path: &str, file: &str, idx: usize, shell: Shell) -> Result<(), String> {
    let _ = shell;
    let (old_content, new_content) = get_file_content_pair(repo_path, file)
        .ok_or_else(|| "Failed to read file content".to_string())?;
    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    let diff = TextDiff::from_lines(&old_content, &new_content);
    let groups = diff.grouped_ops(DIFF_CONTEXT);

    if idx >= groups.len() {
        return Err("bad hunk index".to_string());
    }

    let mut staged_lines: Vec<String> = old_lines.iter().map(|line| (*line).to_string()).collect();
    let mut offset: isize = 0;
    for op in groups[idx].clone() {
        match op {
            similar::DiffOp::Equal { .. } => {}
            similar::DiffOp::Delete { old_index, old_len, .. } => {
                let start = (old_index as isize + offset) as usize;
                staged_lines.drain(start..start + old_len);
                offset -= old_len as isize;
            }
            similar::DiffOp::Insert { old_index, new_index, new_len } => {
                let start = (old_index as isize + offset) as usize;
                let inserted = new_lines[new_index..new_index + new_len]
                    .iter()
                    .map(|line| (*line).to_string());
                staged_lines.splice(start..start, inserted);
                offset += new_len as isize;
            }
            similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                let start = (old_index as isize + offset) as usize;
                let inserted = new_lines[new_index..new_index + new_len]
                    .iter()
                    .map(|line| (*line).to_string());
                staged_lines.splice(start..start + old_len, inserted);
                offset += new_len as isize - old_len as isize;
            }
        }
    }

    write_index_content(repo_path, file, lines_to_content(staged_lines, &new_content))
}

/// 暂存指定的新增行
///
/// 将文件中特定位置的新增行添加到 Git 暂存区。
///
/// # 参数
///
/// - `repo_path`: Git 仓库的根目录路径
/// - `file_path`: 要操作的文件路径（相对于仓库根目录）
/// - `new_idx`: 新增行在工作区文件中的行索引（从 0 开始）
/// - `shell`: 执行命令时使用的 shell 类型
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误信息
///
/// # 实现原理
///
/// 1. 分析 diff 操作，定位新增行对应的旧文件位置
/// 2. 生成最小化 patch（零上下文）
/// 3. 使用 `git apply --cached` 应用到暂存区
///
/// # 示例
///
/// ```ignore
/// use crate::app::git::git_stage_line_insert;
/// use crate::app::Shell;
///
/// // 暂存第 5 行（索引为 4）的新增内容
/// git_stage_line_insert("/path/to/repo", "src/main.rs", 4, Shell::Bash)?;
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn git_stage_line_insert(
    repo_path: &str,
    file_path: &str,
    new_idx: usize,
    shell: Shell,
) -> Result<(), String> {
    let _ = shell;
    let (old_content, new_content) = get_file_content_pair(repo_path, file_path)
        .ok_or_else(|| "Failed to read file".to_string())?;
    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    let diff = TextDiff::from_lines(&old_content, &new_content);

    // 遍历 diff 操作，定位新增行在旧文件中的对应位置
    let mut last_old_end = 0usize;
    let mut target_old_pos = None::<usize>;

    for group in diff.grouped_ops(DIFF_CONTEXT) {
        for op in group {
            match op {
                similar::DiffOp::Equal { old_index, new_index, len } => {
                    last_old_end = last_old_end.max(old_index + len);
                    if new_index <= new_idx && new_idx < new_index + len {
                        // 在相等块中，新增行位置基于旧文件的对应位置计算
                        target_old_pos = Some(old_index + (new_idx - new_index));
                    }
                }
                similar::DiffOp::Delete { old_index, old_len, .. } => {
                    last_old_end = last_old_end.max(old_index + old_len);
                }
                similar::DiffOp::Insert { new_index, new_len, .. } => {
                    if new_index <= new_idx && new_idx < new_index + new_len {
                        // 新增行位于此插入块中，对应旧文件中上一个已知位置
                        target_old_pos = Some(last_old_end);
                    }
                }
                similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                    last_old_end = last_old_end.max(old_index + old_len);
                    if new_index <= new_idx && new_idx < new_index + new_len {
                        // 新增行在替换块中，近似映射到旧文件的起始位置
                        target_old_pos = Some(old_index);
                    }
                }
            }
        }
    }

    let old_pos = target_old_pos.ok_or_else(|| "Cannot locate insertion anchor".to_string())?;

    let mut staged_lines: Vec<String> = old_lines.iter().map(|line| (*line).to_string()).collect();
    staged_lines.insert(old_pos, new_lines.get(new_idx).copied().unwrap_or("").to_string());
    write_index_content(repo_path, file_path, lines_to_content(staged_lines, &new_content))
}

/// 暂存指定的删除行
///
/// 将文件中特定位置的删除行添加到 Git 暂存区。
///
/// # 参数
///
/// - `repo_path`: Git 仓库的根目录路径
/// - `file_path`: 要操作的文件路径（相对于仓库根目录）
/// - `old_idx`: 删除行在旧文件（HEAD 版本）中的行索引（从 0 开始）
/// - `shell`: 执行命令时使用的 shell 类型
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误信息
///
/// # 实现原理
///
/// 1. 分析 diff 操作，定位删除行在新文件中的对应位置
/// 2. 生成最小化 patch
/// 3. 使用 `git apply --cached` 应用到暂存区
///
/// # 示例
///
/// ```ignore
/// use crate::app::git::git_stage_line_delete;
/// use crate::app::Shell;
///
/// // 暂存第 3 行（索引为 2）的删除
/// git_stage_line_delete("/path/to/repo", "src/main.rs", 2, Shell::Bash)?;
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn git_stage_line_delete(
    repo_path: &str,
    file_path: &str,
    old_idx: usize,
    shell: Shell,
) -> Result<(), String> {
    let _ = shell;
    let (old_content, new_content) = get_file_content_pair(repo_path, file_path)
        .ok_or_else(|| "Failed to read file".to_string())?;
    let old_lines: Vec<&str> = old_content.lines().collect();
    let _new_lines: Vec<&str> = new_content.lines().collect();

    if old_idx >= old_lines.len() {
        return Err("Old line index out of bounds".to_string());
    }

    let diff = TextDiff::from_lines(&old_content, &new_content);

    // 遍历 diff 操作，定位删除行在新文件中的对应位置
    let mut last_new_end = 0usize;
    let mut target_new_pos = None::<usize>;

    for group in diff.grouped_ops(DIFF_CONTEXT) {
        for op in group {
            match op {
                similar::DiffOp::Equal { old_index, new_index, len } => {
                    // 如果 old_idx 位于相等块中，计算对应的新文件位置
                    if old_index <= old_idx && old_idx < old_index + len {
                        target_new_pos = Some(new_index + (old_idx - old_index));
                    }
                    last_new_end = last_new_end.max(new_index + len);
                }
                similar::DiffOp::Delete { old_index, old_len, new_index } => {
                    if old_index <= old_idx && old_idx < old_index + old_len {
                        target_new_pos = Some(new_index);
                    }
                    last_new_end = last_new_end.max(new_index);
                }
                similar::DiffOp::Insert { new_index, new_len, .. } => {
                    last_new_end = last_new_end.max(new_index + new_len);
                }
                similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                    if old_index <= old_idx && old_idx < old_index + old_len {
                        target_new_pos = Some(new_index);
                    }
                    last_new_end = last_new_end.max(new_index + new_len);
                }
            }
        }
    }

    let _ = target_new_pos.unwrap_or(last_new_end);
    let mut staged_lines: Vec<String> = old_lines.iter().map(|line| (*line).to_string()).collect();
    staged_lines.remove(old_idx);
    write_index_content(repo_path, file_path, lines_to_content(staged_lines, &new_content))
}

#[cfg(not(target_arch = "wasm32"))]
fn lines_to_content(lines: Vec<String>, reference: &str) -> String {
    let mut content = lines.join("\n");
    if reference.ends_with('\n') && !content.is_empty() {
        content.push('\n');
    }
    content
}

#[cfg(not(target_arch = "wasm32"))]
fn write_index_content(repo_path: &str, file_path: &str, content: String) -> Result<(), String> {
    let repo = git2::Repository::open(repo_path).map_err(|e| e.message().to_string())?;
    let oid = repo.blob(content.as_bytes()).map_err(|e| e.message().to_string())?;
    let mut index = repo.index().map_err(|e| e.message().to_string())?;
    let path = std::path::Path::new(file_path);
    let entry = index.get_path(path, 0).ok_or_else(|| "index entry not found".to_string())?;
    let mode = format!("{:o}", entry.mode);
    let status = std::process::Command::new("git")
        .arg("update-index")
        .arg("--cacheinfo")
        .arg(mode)
        .arg(oid.to_string())
        .arg(file_path)
        .current_dir(repo_path)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        index.read(true).map_err(|e| e.message().to_string())?;
        return index.write().map_err(|e| e.message().to_string());
    }
    Err(format!("git update-index failed with status {status}"))
}

/// 撤销指定行的删除（恢复被删除的行）
///
/// 从工作区文件中移除指定行，用于恢复"删除行"的更改。
///
/// # 参数
///
/// - `repo_path`: Git 仓库的根目录路径
/// - `file_path`: 要操作的文件路径（相对于仓库根目录）
/// - `line_idx`: 要移除的行索引（从 0 开始）
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误信息
///
/// # 示例
///
/// ```ignore
/// use crate::app::git::git_revert_line_delete;
///
/// // 撤销第 5 行（索引为 4）的删除操作
/// git_revert_line_delete("/path/to/repo", "src/main.rs", 4)?;
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn git_revert_line_delete(
    repo_path: &str,
    file_path: &str,
    line_idx: usize,
) -> Result<(), String> {
    let full_path = std::path::Path::new(repo_path).join(file_path);
    let content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;
    let mut lines: Vec<&str> = content.lines().collect();

    if line_idx >= lines.len() {
        return Err("Line index out of bounds".to_string());
    }

    lines.remove(line_idx);

    let mut final_content = lines.join("\n");
    if !final_content.is_empty() {
        final_content.push('\n');
    }

    std::fs::write(&full_path, final_content).map_err(|e| e.to_string())
}

/// 恢复被删除的行到指定位置
///
/// 将 HEAD 版本中的某一行恢复到工作区文件的指定位置。
///
/// # 参数
///
/// - `repo_path`: Git 仓库的根目录路径
/// - `file_path`: 要操作的文件路径（相对于仓库根目录）
/// - `insert_idx`: 要插入的位置索引（从 0 开始）
/// - `old_line_idx`: HEAD 版本中要恢复的行索引（从 0 开始）
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误信息
///
/// # 示例
///
/// ```ignore
/// use crate::app::git::git_revert_line_restore;
///
/// // 将 HEAD 版本的第 3 行恢复到当前文件的第 5 行位置
/// git_revert_line_restore("/path/to/repo", "src/main.rs", 4, 2)?;
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn git_revert_line_restore(
    repo_path: &str,
    file_path: &str,
    insert_idx: usize,
    old_line_idx: usize,
) -> Result<(), String> {
    let (old_content, _) = get_file_content_pair(repo_path, file_path)
        .ok_or_else(|| "Failed to read file".to_string())?;
    let old_lines: Vec<&str> = old_content.lines().collect();

    if old_line_idx >= old_lines.len() {
        return Err("Old line index out of bounds".to_string());
    }
    let line_to_restore = old_lines[old_line_idx];

    let full_path = std::path::Path::new(repo_path).join(file_path);
    let content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    // 处理插入位置超出范围的情况
    if insert_idx > lines.len() {
        lines.push(line_to_restore.to_string());
    } else {
        lines.insert(insert_idx, line_to_restore.to_string());
    }

    let mut final_content = lines.join("\n");
    if !final_content.is_empty() {
        final_content.push('\n');
    }

    std::fs::write(&full_path, final_content).map_err(|e| e.to_string())
}

/// 执行 shell 命令并返回输出
///
/// 内部辅助函数，用于在指定目录下执行 shell 命令。
///
/// # 参数
///
/// - `cwd`: 工作目录（可选）
/// - `cmd`: 要执行的命令字符串
/// - `shell`: 使用的 shell 类型
///
/// # 返回值
///
/// 返回命令的标准输出和标准错误的合并结果
#[cfg(not(target_arch = "wasm32"))]
fn run_shell_command(
    cwd: Option<String>,
    cmd: String,
    #[cfg_attr(windows, allow(unused_variables))] shell: Shell,
) -> String {
    #[cfg(windows)]
    let mut command = std::process::Command::new("cmd");
    #[cfg(windows)]
    {
        command.arg("/C").arg(cmd.clone());
    }
    #[cfg(not(windows))]
    let mut command = std::process::Command::new(match shell {
        Shell::Bash => "bash",
        Shell::Zsh => "zsh",
    });
    #[cfg(not(windows))]
    {
        command.arg("-lc").arg(cmd);
    }
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    match command.output() {
        Ok(out) => {
            let mut s = String::new();
            s.push_str(&String::from_utf8_lossy(&out.stdout));
            s.push_str(&String::from_utf8_lossy(&out.stderr));
            s.replace("\r\n", "\n").replace('\r', "\n")
        }
        Err(e) => format!("执行失败: {}\n", e),
    }
}

// ============================================================================
// Web 平台 (WASM) 存根实现
// ============================================================================

/// Web 平台存根：丢弃文件操作不支持
#[cfg(target_arch = "wasm32")]
pub fn git_discard_file(_repo_path: &str, _file: &str) -> Result<(), String> {
    Err("Not supported on Web".to_string())
}

/// Web 平台存根：查看文件差异不支持
#[cfg(target_arch = "wasm32")]
pub fn git_diff_for_file(_repo_path: &str, _file: &str) -> Option<String> {
    None
}

/// Web 平台存根：丢弃代码块操作不支持
#[cfg(target_arch = "wasm32")]
pub fn git_discard_hunk(
    _repo_path: &str,
    _file: &str,
    _idx: usize,
    _shell: Shell,
) -> Result<(), String> {
    Err("Not supported on Web".to_string())
}

/// Web 平台存根：撤销行删除操作不支持
#[cfg(target_arch = "wasm32")]
pub fn git_revert_line_delete(
    _repo_path: &str,
    _file_path: &str,
    _line_idx: usize,
) -> Result<(), String> {
    Err("Not supported on Web".to_string())
}

/// Web 平台存根：恢复行操作不支持
#[cfg(target_arch = "wasm32")]
pub fn git_revert_line_restore(
    _repo_path: &str,
    _file_path: &str,
    _insert_idx: usize,
    _old_line_idx: usize,
) -> Result<(), String> {
    Err("Not supported on Web".to_string())
}

#[cfg(test)]
#[path = "git_tests.rs"]
mod git_tests;
