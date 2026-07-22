//! ACP 代理进程组清理和退出摘要。

use super::*;
#[cfg(unix)]
use std::collections::{HashMap, HashSet};
#[cfg(unix)]
use std::process::Command as StdCommand;

#[derive(Clone, Debug, Default)]
pub(super) struct ProcessCleanupTargets {
    pub(super) process_groups: Vec<u32>,
    pub(super) processes: Vec<u32>,
}

impl ProcessCleanupTargets {
    fn from_root(process_id: Option<u32>) -> Self {
        let mut targets = Self::default();
        if let Some(process_id) = process_id {
            targets.push_process_group(process_id);
            targets.push_process(process_id);
        }
        targets
    }

    pub(super) fn extend(&mut self, other: Self) {
        for process_group_id in other.process_groups {
            self.push_process_group(process_group_id);
        }
        for process_id in other.processes {
            self.push_process(process_id);
        }
    }

    fn push_process_group(&mut self, process_group_id: u32) {
        if process_group_id == 0 || self.process_groups.contains(&process_group_id) {
            return;
        }
        self.process_groups.push(process_group_id);
    }

    fn push_process(&mut self, process_id: u32) {
        if process_id == 0 || self.processes.contains(&process_id) {
            return;
        }
        self.processes.push(process_id);
    }
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ProcessTableEntry {
    process_id: u32,
    parent_process_id: u32,
    process_group_id: u32,
}

/// 捕获代理进程及其当前子孙进程的清理目标。
///
/// 部分 ACP 代理会再启动 node/codegraph 等辅助进程，并把这些进程放入新的进程组。
/// 只清理最初的代理进程组会留下这些辅助进程，因此这里先记录整棵子进程树中出现过的
/// pid 和 pgid，后续关闭时逐一收敛。
pub(super) fn process_cleanup_targets(process_id: Option<u32>) -> ProcessCleanupTargets {
    let mut targets = ProcessCleanupTargets::from_root(process_id);

    #[cfg(unix)]
    if let Some(process_id) = process_id
        && let Some(entries) = load_process_table_entries()
    {
        targets.extend(process_cleanup_targets_from_entries(process_id, &entries));
    }

    targets
}

#[cfg(unix)]
fn load_process_table_entries() -> Option<Vec<ProcessTableEntry>> {
    let output = StdCommand::new("ps").args(["-axo", "pid=,ppid=,pgid="]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let output = String::from_utf8_lossy(&output.stdout);
    Some(parse_process_table_entries(&output))
}

#[cfg(unix)]
pub(super) fn parse_process_table_entries(input: &str) -> Vec<ProcessTableEntry> {
    input
        .lines()
        .filter_map(|line| {
            let mut columns = line.split_whitespace();
            let process_id = columns.next()?.parse::<u32>().ok()?;
            let parent_process_id = columns.next()?.parse::<u32>().ok()?;
            let process_group_id = columns.next()?.parse::<u32>().ok()?;
            Some(ProcessTableEntry { process_id, parent_process_id, process_group_id })
        })
        .collect()
}

#[cfg(unix)]
pub(super) fn process_cleanup_targets_from_entries(
    root_process_id: u32,
    entries: &[ProcessTableEntry],
) -> ProcessCleanupTargets {
    let mut targets = ProcessCleanupTargets::from_root(Some(root_process_id));
    let mut children_by_parent: HashMap<u32, Vec<ProcessTableEntry>> = HashMap::new();
    for entry in entries {
        children_by_parent.entry(entry.parent_process_id).or_default().push(*entry);
    }

    let mut seen = HashSet::new();
    seen.insert(root_process_id);
    let mut pending = vec![root_process_id];
    for entry in entries {
        if entry.process_group_id != root_process_id || !seen.insert(entry.process_id) {
            continue;
        }
        targets.push_process(entry.process_id);
        targets.push_process_group(entry.process_group_id);
        pending.push(entry.process_id);
    }

    while let Some(parent_process_id) = pending.pop() {
        let Some(children) = children_by_parent.get(&parent_process_id) else {
            continue;
        };
        for child in children {
            if !seen.insert(child.process_id) {
                continue;
            }
            targets.push_process(child.process_id);
            targets.push_process_group(child.process_group_id);
            pending.push(child.process_id);
        }
    }

    targets
}

/// 清理代理进程树。
///
/// 调用方应尽量在开始关闭前捕获目标；即使父进程稍后退出导致孙进程被系统接管，
/// 这里仍会按之前记录的 pid/pgid 做最后收敛。
pub(super) async fn cleanup_process_targets(targets: ProcessCleanupTargets) {
    send_terminate_signal_to_process_targets(&targets);
    tokio::time::sleep(Duration::from_millis(150)).await;
    send_kill_signal_to_process_targets(&targets);
}

pub(super) fn send_terminate_signal_to_process_targets(targets: &ProcessCleanupTargets) {
    for process_group_id in &targets.process_groups {
        send_terminate_signal_to_process_group(Some(*process_group_id));
    }
    for process_id in &targets.processes {
        send_terminate_signal_to_process(Some(*process_id));
    }
}

pub(super) fn send_kill_signal_to_process_targets(targets: &ProcessCleanupTargets) {
    for process_group_id in &targets.process_groups {
        send_kill_signal_to_process_group(Some(*process_group_id));
    }
    for process_id in &targets.processes {
        send_kill_signal_to_process(Some(*process_id));
    }
}

/// 向 Unix 进程组发送 `SIGTERM`。
///
/// `process_group_id` 为 `None` 时不会执行任何操作；发送失败会被忽略，因为
/// 进程可能已经退出。
#[cfg(unix)]
pub(super) fn send_terminate_signal_to_process_group(process_group_id: Option<u32>) {
    let _ = send_signal_to_process_group(process_group_id, libc::SIGTERM);
}

/// 非 Unix 平台的 `SIGTERM` 占位实现。
#[cfg(not(unix))]
pub(super) fn send_terminate_signal_to_process_group(_process_group_id: Option<u32>) {}

#[cfg(unix)]
fn send_terminate_signal_to_process(process_id: Option<u32>) {
    let _ = send_signal_to_process(process_id, libc::SIGTERM);
}

#[cfg(not(unix))]
fn send_terminate_signal_to_process(_process_id: Option<u32>) {}

/// 向 Unix 进程组发送 `SIGKILL`。
///
/// 用于温和终止超时后的兜底清理，确保代理子进程不会继续持有工作区资源。
#[cfg(unix)]
pub(super) fn send_kill_signal_to_process_group(process_group_id: Option<u32>) {
    let _ = send_signal_to_process_group(process_group_id, libc::SIGKILL);
}

/// 非 Unix 平台的 `SIGKILL` 占位实现。
#[cfg(not(unix))]
pub(super) fn send_kill_signal_to_process_group(_process_group_id: Option<u32>) {}

#[cfg(unix)]
fn send_kill_signal_to_process(process_id: Option<u32>) {
    let _ = send_signal_to_process(process_id, libc::SIGKILL);
}

#[cfg(not(unix))]
fn send_kill_signal_to_process(_process_id: Option<u32>) {}

#[cfg(unix)]
fn send_signal_to_process_group(process_group_id: Option<u32>, signal: i32) -> bool {
    let Some(process_group_id) = process_group_id else {
        return false;
    };
    let result = unsafe { libc::kill(-(process_group_id as i32), signal) };
    if result == 0 {
        return true;
    }
    matches!(std::io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH))
}

#[cfg(unix)]
fn send_signal_to_process(process_id: Option<u32>, signal: i32) -> bool {
    let Some(process_id) = process_id else {
        return false;
    };
    let result = unsafe { libc::kill(process_id as i32, signal) };
    if result == 0 {
        return true;
    }
    matches!(std::io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH))
}

/// 从子进程退出状态构造生命周期摘要。
///
/// 返回值包含退出码和 Unix 信号名；没有退出状态时字段为空。
pub(super) fn child_exit_summary(status: Option<&ExitStatus>) -> ChildExitSummary {
    ChildExitSummary {
        exit_code: status.and_then(ExitStatus::code),
        signal: exit_signal_name(status),
    }
}

#[cfg(unix)]
fn exit_signal_name(status: Option<&ExitStatus>) -> Option<String> {
    use std::os::unix::process::ExitStatusExt;

    status.and_then(|value| value.signal()).map(|signal| format!("SIG{signal}"))
}

#[cfg(not(unix))]
fn exit_signal_name(_status: Option<&ExitStatus>) -> Option<String> {
    None
}
