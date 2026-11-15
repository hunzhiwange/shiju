//! Shell 路径约束检查，负责把命令参数和重定向路径限制在允许根目录内。

use std::path::{Component, Path, PathBuf};

use directories::BaseDirs;

use crate::shell::ast::{CommandInfo, ParsedCommand, RedirectKind, strip_wrappers};

mod extractors;

/// 重导出 extractors::{PATH_EXTRACTORS, PathArgKind, PathExtractor}，保持外部调用路径稳定。
pub use extractors::{PATH_EXTRACTORS, PathArgKind, PathExtractor};

/// PathCheckResult 枚举描述当前模块中一组明确的状态或分类。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathCheckResult {
    Allowed,
    Blocked { path: PathBuf, reason: String },
}

/// 执行 check_path_constraints 操作，并返回调用方需要的结果。
pub fn check_path_constraints(
    cmd: &ParsedCommand,
    workspace_dir: &Path,
    allowed_roots: &[PathBuf],
) -> PathCheckResult {
    for raw_path in extract_candidate_paths(cmd) {
        if raw_path.starts_with('~') && raw_path != "~" && !raw_path.starts_with("~/") {
            return PathCheckResult::Blocked {
                path: PathBuf::from(raw_path),
                reason: "tilde-user paths are not allowed".into(),
            };
        }
        let Some(resolved) = resolve_path(&raw_path, workspace_dir) else {
            continue;
        };

        if is_dangerous_path(&resolved) && !allows_all_paths(allowed_roots) {
            return PathCheckResult::Blocked {
                path: resolved,
                reason: "Path is on the dangerous paths list".into(),
            };
        }

        if !is_path_allowed(&resolved, workspace_dir, allowed_roots) {
            return PathCheckResult::Blocked {
                path: resolved,
                reason: "Path is outside allowed directories".into(),
            };
        }
    }

    PathCheckResult::Allowed
}

/// 执行 extract_redirect_paths 操作，并返回调用方需要的结果。
pub fn extract_redirect_paths(cmd: &ParsedCommand) -> Vec<PathBuf> {
    command_info(cmd)
        .map(|info| {
            info.redirects
                .iter()
                .filter(|redirect| !redirect.is_fd_duplicate)
                .filter(|redirect| !matches!(redirect.kind, RedirectKind::Heredoc))
                .filter(|redirect| redirect.target != "/dev/null")
                .filter_map(|redirect| resolve_path(&redirect.target, Path::new(".")))
                .collect()
        })
        .unwrap_or_default()
}

fn extract_candidate_paths(cmd: &ParsedCommand) -> Vec<String> {
    let Some(info) = command_info(cmd) else {
        return fallback_candidate_paths(cmd);
    };

    let command = base_command_name(&info.name);
    let args = info.args.clone();
    let mut paths = match command {
        "grep" | "rg" | "ag" | "ack" => extract_search_paths(&args),
        "git" => extract_git_paths(&args),
        "jq" => extract_jq_paths(&args),
        "sed" => extract_sed_paths(&args),
        "find" => extract_find_paths(&args),
        "chmod" | "chown" => extract_mode_target_paths(&args),
        _ => extractors::extract_paths(command, &args),
    };

    for redirect in &info.redirects {
        if redirect.is_fd_duplicate || redirect.target == "/dev/null" {
            continue;
        }
        if matches!(redirect.kind, RedirectKind::Heredoc) {
            continue;
        }
        paths.push(redirect.target.clone());
    }

    paths
}

fn fallback_candidate_paths(cmd: &ParsedCommand) -> Vec<String> {
    let ParsedCommand::Fallback { tokens, .. } = cmd else {
        return Vec::new();
    };
    let Some((command, args)) = tokens.split_first() else {
        return Vec::new();
    };
    extractors::extract_paths(base_command_name(command), args)
}

fn extract_search_paths(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut positional = Vec::new();
    let mut after_double_dash = false;
    let mut index = 0;

    while index < args.len() {
        let arg = args[index].as_str();
        if !after_double_dash && arg == "--" {
            after_double_dash = true;
            index += 1;
            continue;
        }

        if !after_double_dash {
            if let Some(value) = take_flag_value(
                args,
                &mut index,
                &["-f", "--file", "--include", "--exclude", "--glob", "-g"],
            ) {
                out.push(value);
                continue;
            }

            if arg.starts_with('-') && arg != "-" {
                index += 1;
                continue;
            }
        }

        positional.push(arg.to_string());
        index += 1;
    }

    if positional.len() > 1 {
        out.extend(positional.into_iter().skip(1));
    }

    out
}

fn extract_git_paths(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut index = 0;
    while index < args.len() {
        if let Some(value) =
            take_flag_value(args, &mut index, &["-C", "--git-dir", "--work-tree", "--file"])
        {
            out.push(value);
            continue;
        }
        index += 1;
    }
    out
}

fn extract_jq_paths(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut positional = Vec::new();
    let mut index = 0;

    while index < args.len() {
        if let Some(value) =
            take_flag_value(args, &mut index, &["-f", "--from-file", "--rawfile", "--slurpfile"])
        {
            out.push(value);
            if matches!(args[index - 1].as_str(), "--rawfile" | "--slurpfile") && index < args.len()
            {
                index += 1;
            }
            continue;
        }

        let arg = args[index].as_str();
        if arg.starts_with('-') && arg != "-" {
            index += 1;
            continue;
        }
        positional.push(arg.to_string());
        index += 1;
    }

    if positional.len() > 1 {
        out.push(positional[1].clone());
    }

    out
}

fn extract_sed_paths(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut positional = Vec::new();
    let mut script_seen = false;
    let mut index = 0;

    while index < args.len() {
        let arg = args[index].as_str();
        if arg == "-e" {
            index += 2;
            script_seen = true;
            continue;
        }
        if arg == "-f" {
            if let Some(value) = args.get(index + 1) {
                out.push(value.clone());
            }
            index += 2;
            script_seen = true;
            continue;
        }
        if arg == "-i" {
            if args.get(index + 1).is_some_and(|value| value.is_empty()) {
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        if arg.starts_with("-i") || matches!(arg, "-E" | "-n" | "-r" | "--") {
            index += 1;
            continue;
        }
        if arg.starts_with('-') && arg != "-" {
            index += 1;
            continue;
        }

        if script_seen {
            positional.push(arg.to_string());
        } else {
            script_seen = true;
        }
        index += 1;
    }

    out.extend(positional);
    out
}

fn extract_find_paths(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for arg in args {
        if arg == "--" {
            continue;
        }
        if arg.starts_with('-') || matches!(arg.as_str(), "!" | "(" | ")") {
            break;
        }
        out.push(arg.clone());
    }
    out
}

fn extract_mode_target_paths(args: &[String]) -> Vec<String> {
    args.iter().skip(1).filter(|arg| !arg.starts_with('-')).cloned().collect()
}

fn take_flag_value(args: &[String], index: &mut usize, flags: &[&str]) -> Option<String> {
    let arg = args.get(*index)?.as_str();
    for flag in flags {
        if arg == *flag {
            let value = args.get(*index + 1)?.clone();
            *index += 2;
            return Some(value);
        }
        if let Some(value) = arg.strip_prefix(&format!("{flag}=")) {
            *index += 1;
            return Some(value.to_string());
        }
        if !flag.starts_with("--") && arg.starts_with(flag) && arg.len() > flag.len() {
            *index += 1;
            return Some(arg[flag.len()..].to_string());
        }
    }
    None
}

fn command_info(cmd: &ParsedCommand) -> Option<CommandInfo> {
    match cmd {
        ParsedCommand::Ast(_, info) => Some(strip_wrappers(info)),
        ParsedCommand::Fallback { .. } => None,
    }
}

fn base_command_name(command: &str) -> &str {
    command.rsplit('/').next().unwrap_or(command)
}

fn resolve_path(raw: &str, workspace_dir: &Path) -> Option<PathBuf> {
    if raw.is_empty() || raw == "-" {
        return None;
    }

    let expanded = if raw == "~" || raw.starts_with("~/") {
        PathBuf::from(shellexpand::tilde(raw).into_owned())
    } else {
        PathBuf::from(raw)
    };

    let joined = if expanded.is_absolute() { expanded } else { workspace_dir.join(expanded) };

    Some(normalize_path(&joined))
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = if path.is_absolute() {
        PathBuf::from(Component::RootDir.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::RootDir => {
                normalized = PathBuf::from(Component::RootDir.as_os_str());
            }
            Component::Normal(part) => normalized.push(part),
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
        }
    }

    normalized
}

fn is_path_allowed(resolved: &Path, workspace_dir: &Path, allowed_roots: &[PathBuf]) -> bool {
    let workspace_dir = normalize_path(workspace_dir);
    if resolved.starts_with(&workspace_dir) {
        return true;
    }

    allowed_roots.iter().map(|root| normalize_path(root)).any(|root| resolved.starts_with(root))
}

fn allows_all_paths(allowed_roots: &[PathBuf]) -> bool {
    allowed_roots.iter().map(|root| normalize_path(root)).any(|root| root == Path::new("/"))
}

fn is_dangerous_path(path: &Path) -> bool {
    const DANGEROUS_PREFIXES: &[&str] =
        &["/etc/passwd", "/etc/shadow", "/etc/ssh", "/proc", "/sys", "/dev", "/root/.ssh"];

    if DANGEROUS_PREFIXES.iter().map(Path::new).any(|prefix| path.starts_with(prefix)) {
        return true;
    }

    let user_ssh = BaseDirs::new().map(|dirs| dirs.home_dir().join(".ssh"));
    if user_ssh.as_ref().is_some_and(|prefix| path.starts_with(prefix)) {
        return true;
    }

    let mut components = path.components();
    matches!(components.next(), Some(Component::RootDir))
        && matches!(components.next(), Some(Component::Normal(first)) if first == std::ffi::OsStr::new("home"))
        && matches!(components.next(), Some(Component::Normal(_)))
        && matches!(components.next(), Some(Component::Normal(last)) if last == std::ffi::OsStr::new(".ssh"))
}

#[cfg(test)]
#[path = "extractors_tests.rs"]
mod extractors_tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
