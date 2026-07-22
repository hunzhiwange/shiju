//! 工作区健康检查。
//!
//! 本模块检查配置中的工作区目录是否存在、是否可写、磁盘空间是否充足，以及
//! 常见项目说明文件是否存在。检查会尽量局部化副作用，只创建并立即删除一个
//! probe 文件来验证写权限。

use super::DiagItem;
use crate::app::agent::config::Config;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 检查当前工作区并追加诊断项。
///
/// 参数：
/// - `config`：包含工作区路径的运行配置。
/// - `items`：收集诊断结果的输出列表。
///
/// 错误处理：
/// 目录缺失、写入失败或空间不足会被转换为诊断项；函数本身不返回错误，以便
/// doctor 能继续展示同一轮中的其他检查结果。
pub(super) fn check_workspace(config: &Config, items: &mut Vec<DiagItem>) {
    check_workspace_with_disk_probe(config, items, disk_available_mb);
}

/// 使用指定磁盘空间探针检查当前工作区并追加诊断项。
pub(super) fn check_workspace_with_disk_probe(
    config: &Config,
    items: &mut Vec<DiagItem>,
    disk_probe: fn(&Path) -> Option<u64>,
) {
    let cat = "workspace";
    let workspace_dir = &config.workspace_dir;

    if workspace_dir.exists() {
        items.push(DiagItem::ok(cat, format!("directory exists: {}", workspace_dir.display())));
    } else {
        items.push(DiagItem::error(cat, format!("directory missing: {}", workspace_dir.display())));
        return;
    }

    let probe = workspace_probe_path(workspace_dir);
    // 使用 create_new 避免覆盖用户文件；probe 文件名包含进程号和时间戳，冲突概率低，
    // 即使残留也只会出现在工作区根目录并带有明确前缀。
    match std::fs::OpenOptions::new().write(true).create_new(true).open(&probe) {
        Ok(mut probe_file) => {
            let write_result = probe_file.write_all(b"probe");
            drop(probe_file);
            let _ = std::fs::remove_file(&probe);
            match write_result {
                Ok(()) => items.push(DiagItem::ok(cat, "directory is writable")),
                Err(err) => {
                    items.push(DiagItem::error(cat, format!("directory write probe failed: {err}")))
                }
            }
        }
        Err(err) => {
            items.push(DiagItem::error(cat, format!("directory is not writable: {err}")));
        }
    }

    append_disk_space_check(disk_probe(workspace_dir), cat, items);

    check_file_exists(workspace_dir, "SOUL.md", false, cat, items);
    check_file_exists(workspace_dir, "AGENTS.md", false, cat, items);
}

/// 根据可用磁盘空间追加对应诊断项。
pub(super) fn append_disk_space_check(
    available_mb: Option<u64>,
    category: &'static str,
    items: &mut Vec<DiagItem>,
) {
    if let Some(avail_mb) = available_mb {
        if avail_mb >= 100 {
            items.push(DiagItem::ok(category, format!("disk space: {avail_mb} MB available")));
        } else {
            items.push(DiagItem::warn(
                category,
                format!("low disk space: only {avail_mb} MB available"),
            ));
        }
    }
}

/// 检查指定文件是否存在并追加对应诊断项。
///
/// 参数：
/// - `base`：待检查文件所在目录。
/// - `name`：文件名。
/// - `required`：是否将缺失视为错误。
/// - `category`：诊断分类。
/// - `items`：收集诊断结果的输出列表。
pub(super) fn check_file_exists(
    base: &Path,
    name: &str,
    required: bool,
    category: &'static str,
    items: &mut Vec<DiagItem>,
) {
    let path = base.join(name);
    if path.is_file() {
        items.push(DiagItem::ok(category, format!("{name} present")));
    } else if required {
        items.push(DiagItem::error(category, format!("{name} missing")));
    } else {
        items.push(DiagItem::warn(category, format!("{name} not found (optional)")));
    }
}

/// 查询指定路径所在文件系统的可用空间，单位为 MB。
///
/// 参数：
/// - `path`：用于传给 `df -m` 的路径。
///
/// 返回值：
/// 成功解析时返回可用 MB；命令不可用、执行失败或输出格式不符合预期时返回 `None`。
fn disk_available_mb(path: &Path) -> Option<u64> {
    let output = std::process::Command::new("df").arg("-m").arg(path).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_df_available_mb(&stdout)
}

/// 从 `df -m` 输出中解析可用空间列。
///
/// 参数：
/// - `stdout`：`df -m` 的标准输出。
///
/// 返回值：
/// 返回最后一条非空数据行的可用空间列；解析失败时返回 `None`。
pub(super) fn parse_df_available_mb(stdout: &str) -> Option<u64> {
    let line = stdout.lines().rev().find(|line| !line.trim().is_empty())?;
    let available = line.split_whitespace().nth(3)?;
    available.parse::<u64>().ok()
}

/// 生成工作区写权限探测文件路径。
///
/// 参数：
/// - `workspace_dir`：工作区根目录。
///
/// 返回值：
/// 返回位于工作区根目录、带 `.vibewindow_doctor_probe_` 前缀的临时文件路径。
pub(super) fn workspace_probe_path(workspace_dir: &Path) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    workspace_dir.join(format!(".vibewindow_doctor_probe_{}_{}", std::process::id(), nanos))
}
