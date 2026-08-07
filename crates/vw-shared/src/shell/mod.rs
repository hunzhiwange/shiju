//! Shell 管理模块
//!
//! 本模块负责 Shell 环境的检测、选择和进程管理，为代理系统提供可靠的命令执行环境。
//!
//! # 核心功能
//!
//! - **Shell 自动检测**：根据操作系统和环境变量自动选择合适的 Shell
//! - **Shell 黑名单机制**：排除不兼容的 Shell（如 fish、nushell）
//! - **进程树终止**：跨平台终止进程及其所有子进程
//!
//! # Shell 选择优先级
//!
//! 1. 优先使用环境变量 `SHELL` 指定的 Shell（除非在黑名单中）
//! 2. Windows：尝试 Git Bash → COMSPEC (cmd.exe)
//! 3. macOS：默认使用 `/bin/zsh`
//! 4. 其他 Unix：尝试 `bash` → `/bin/sh`

use once_cell::sync::Lazy;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::Duration;

const SIGKILL_TIMEOUT_MS: u64 = 200;

const GIT_DEFAULT_ENV: &[(&str, &str)] =
    &[("GIT_PAGER", "cat"), ("GIT_MERGE_AUTOEDIT", "no"), ("GIT_EDITOR", "true")];

static BLACKLIST: Lazy<std::collections::HashSet<&'static str>> =
    Lazy::new(|| ["fish", "nu"].into_iter().collect());

#[cfg(not(windows))]
const COMMON_COMMAND_DIRS: &[&str] = &[
    "/opt/homebrew/bin",
    "/opt/homebrew/sbin",
    "/usr/local/bin",
    "/usr/local/sbin",
    "/usr/bin",
    "/bin",
    "/usr/sbin",
    "/sbin",
];

#[cfg(windows)]
const COMMON_COMMAND_DIRS: &[&str] = &[];

fn basename(p: &str) -> Option<String> {
    Path::new(p).file_name().and_then(|s| s.to_str()).map(|s| s.to_string())
}

#[cfg_attr(not(test), allow(dead_code))]
fn which(program: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let p = dir.join(program);
        if p.is_file() {
            return Some(p);
        }
        #[cfg(windows)]
        {
            let p = dir.join(format!("{program}.exe"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

fn user_home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

#[cfg(not(windows))]
fn shell_profile_paths(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".zshenv"),
        home.join(".zprofile"),
        home.join(".zshrc"),
        home.join(".bash_profile"),
        home.join(".bashrc"),
        home.join(".profile"),
    ]
}

#[cfg(not(windows))]
fn path_separator() -> char {
    ':'
}

#[cfg(not(windows))]
fn strip_matching_quotes(value: &str) -> &str {
    let bytes = value.as_bytes();
    if value.len() >= 2
        && ((bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\''))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

#[cfg(not(windows))]
fn replace_shell_home_and_path_vars(value: &str, home: &str, current_path: &str) -> String {
    let mut expanded = strip_matching_quotes(value.trim()).to_string();
    expanded = expanded.replace("${HOME}", home).replace("$HOME", home);
    expanded = expanded.replace("${PATH}", current_path).replace("$PATH", current_path);
    if expanded == "~" {
        expanded = home.to_string();
    } else if let Some(rest) = expanded.strip_prefix("~/") {
        expanded = format!("{home}/{rest}");
    }
    expanded
}

#[cfg(not(windows))]
fn extract_env_assignment_rhs<'a>(line: &'a str, env_name: &str) -> Option<&'a str> {
    let candidate = line.trim().strip_prefix("export ").unwrap_or(line.trim()).trim();
    let (lhs, rhs) = candidate.split_once('=')?;
    if lhs.trim() == env_name { Some(rhs.trim()) } else { None }
}

#[cfg(not(windows))]
fn extract_path_assignment_rhs(line: &str) -> Option<&str> {
    extract_env_assignment_rhs(line, "PATH")
}

#[cfg(not(windows))]
fn brew_shellenv_dirs(line: &str) -> Option<Vec<PathBuf>> {
    if !line.contains("brew shellenv") {
        return None;
    }

    let prefix = if line.contains("/opt/homebrew/") {
        Some("/opt/homebrew")
    } else if line.contains("/usr/local/") {
        Some("/usr/local")
    } else {
        None
    }?;

    Some(vec![PathBuf::from(format!("{prefix}/bin")), PathBuf::from(format!("{prefix}/sbin"))])
}

fn push_unique_path(paths: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !candidate.is_dir() {
        return;
    }
    push_known_path(paths, candidate);
}

/// 在不做存在性探测的前提下去重追加目录。
///
/// 用于已知稳定的兜底目录：即便目录暂时不可探测，也应保留在 PATH 中，缺失的目录
/// 对查找无害，而漏掉关键目录会破坏 shebang 脚本的命令解析。
fn push_known_path(paths: &mut Vec<PathBuf>, candidate: PathBuf) {
    if paths.iter().any(|existing| existing == &candidate) {
        return;
    }
    paths.push(candidate);
}

#[cfg(not(windows))]
fn split_path_value(value: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in value.split(path_separator()) {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }
        push_unique_path(&mut out, PathBuf::from(trimmed));
    }
    out
}

#[cfg(not(windows))]
fn infer_path_from_shell_profiles(home: &Path) -> Option<String> {
    let home_string = home.to_string_lossy().to_string();
    let mut path_value = std::env::var("PATH").unwrap_or_default();
    let mut touched = false;

    for profile in shell_profile_paths(home) {
        let Ok(contents) = std::fs::read_to_string(&profile) else {
            continue;
        };

        for raw_line in contents.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(dirs) = brew_shellenv_dirs(line) {
                let mut merged = dirs
                    .into_iter()
                    .filter(|dir| dir.is_dir())
                    .map(|dir| dir.to_string_lossy().to_string())
                    .collect::<Vec<_>>();
                if !path_value.is_empty() {
                    merged.push(path_value.clone());
                }
                path_value = merged.join(":");
                touched = true;
                continue;
            }

            if let Some(rhs) = extract_path_assignment_rhs(line) {
                path_value = replace_shell_home_and_path_vars(rhs, &home_string, &path_value);
                touched = true;
            }
        }
    }

    touched.then_some(path_value)
}

/// 缓存补全后的 PATH。
///
/// 补全 PATH 依赖进程环境与 shell profile，这些在守护进程生命周期内保持稳定。
/// 缓存可以避免在每次子进程启动时重复执行 profile 解析与目录探测；后者在高并发
/// 启动场景下会产生大量 `is_dir` 系统调用，瞬时失败会把 `/opt/homebrew/bin` 这类
/// 关键目录从子进程 PATH 中漏掉，进而导致 `npx` 等 shebang 脚本解析 `node` 失败。
static EFFECTIVE_PATH: Lazy<Option<OsString>> = Lazy::new(build_effective_path);

fn build_effective_path() -> Option<OsString> {
    let mut paths = Vec::new();

    #[cfg(not(windows))]
    {
        if let Some(home) = user_home_dir()
            && let Some(profile_path) = infer_path_from_shell_profiles(&home)
        {
            for entry in split_path_value(&profile_path) {
                push_unique_path(&mut paths, entry);
            }

            for relative in [
                ".local/bin",
                ".cargo/bin",
                ".bun/bin",
                // rustcodegraph: install.sh symlink target dirs + bundle layouts
                ".rustcodegraph/bin",
                ".rustcodegraph/current/bin",
            ] {
                push_unique_path(&mut paths, home.join(relative));
            }
        }
    }

    if let Some(current_path) = std::env::var_os("PATH") {
        for entry in std::env::split_paths(&current_path) {
            push_unique_path(&mut paths, entry);
        }
    }

    // 常见系统目录是稳定的兜底项；即便某次探测瞬时失败也必须保留，否则会把
    // homebrew 等关键目录从子进程 PATH 中漏掉。这里不做 `is_dir` 过滤。
    for dir in COMMON_COMMAND_DIRS {
        push_known_path(&mut paths, PathBuf::from(dir));
    }

    if paths.is_empty() {
        return None;
    }

    std::env::join_paths(paths).ok()
}

/// 返回缓存的补全 PATH。
///
/// 测试构建下跳过缓存，便于用例通过修改进程环境验证 PATH 构建逻辑；生产构建则使用
/// 进程级缓存，消除并发子进程启动时重复探测目录带来的瞬时竞态与开销。
#[cfg(not(test))]
fn cached_effective_path() -> Option<&'static OsString> {
    EFFECTIVE_PATH.as_ref()
}

#[cfg(test)]
fn cached_effective_path() -> Option<OsString> {
    let _ = &*EFFECTIVE_PATH;
    build_effective_path()
}

/// 返回补全后的 PATH 环境变量。
///
/// 该值会合并当前进程 PATH、常见系统目录，以及从用户 `.zshrc` / `.bashrc`
/// / `.profile` 中可安全推断出的 PATH 追加配置。
pub fn effective_path_env() -> Option<String> {
    cached_effective_path().map(|value| value.to_string_lossy().to_string())
}

#[cfg(not(windows))]
fn infer_env_var_from_shell_profiles(home: &Path, env_name: &str) -> Option<String> {
    let env_name = env_name.trim();
    if env_name.is_empty() {
        return None;
    }

    let home_string = home.to_string_lossy().to_string();
    let current_path = std::env::var("PATH").unwrap_or_default();
    let mut resolved = None;

    for profile in shell_profile_paths(home) {
        let Ok(contents) = std::fs::read_to_string(&profile) else {
            continue;
        };

        for raw_line in contents.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(rhs) = extract_env_assignment_rhs(line, env_name) {
                resolved = Some(replace_shell_home_and_path_vars(rhs, &home_string, &current_path));
            }
        }
    }

    resolved.filter(|value| !value.trim().is_empty())
}

#[cfg(windows)]
fn infer_env_var_from_shell_profiles(_home: &Path, _env_name: &str) -> Option<String> {
    None
}

/// 从当前进程环境或用户 shell 配置文件中读取环境变量。
pub fn shell_profile_env_var(env_name: &str) -> Option<String> {
    std::env::var(env_name).ok().filter(|value| !value.trim().is_empty()).or_else(|| {
        let home = user_home_dir()?;
        infer_env_var_from_shell_profiles(&home, env_name)
    })
}

fn which_in_path_value(program: &str, path_value: &std::ffi::OsStr) -> Option<PathBuf> {
    for dir in std::env::split_paths(path_value) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let candidate = dir.join(format!("{program}.exe"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// 使用补全后的 PATH 解析可执行文件路径。
///
/// 当 `program` 是绝对路径或相对路径时，仅在文件存在时返回；当 `program`
/// 是裸命令名时，会在补全后的 PATH 中查找。
pub fn resolve_executable(program: &str) -> Option<PathBuf> {
    let program = program.trim();
    if program.is_empty() {
        return None;
    }

    let candidate = PathBuf::from(program);
    if candidate.is_absolute()
        || candidate.components().count() > 1
        || program.contains('/')
        || program.contains('\\')
    {
        return candidate.is_file().then_some(candidate);
    }

    if let Some(path_value) = std::env::var_os("PATH")
        && let Some(found) = which_in_path_value(program, &path_value)
    {
        return Some(found);
    }

    if let Some(path_value) = cached_effective_path()
        && let Some(found) = which_in_path_value(program, path_value.as_ref())
    {
        return Some(found);
    }

    common_executable(program)
}

fn common_executable(program: &str) -> Option<PathBuf> {
    let common_path = std::env::join_paths(COMMON_COMMAND_DIRS).ok()?;
    which_in_path_value(program, &common_path)
}

fn git_program() -> PathBuf {
    if std::env::var_os("VIBEWINDOW_TEST_PREFER_SYSTEM_GIT").is_some()
        && let Some(found) = common_executable("git")
    {
        return found;
    }

    resolved_program_or_original("git")
}

fn resolved_program_or_original(program: &str) -> PathBuf {
    resolve_executable(program).unwrap_or_else(|| PathBuf::from(program))
}

/// 将补全后的 shell PATH 注入标准命令对象。
pub fn apply_augmented_path_std_command(command: &mut std::process::Command) {
    if let Some(path) = effective_path_env() {
        command.env("PATH", path);
    }
}

/// 将补全后的 shell PATH 注入异步命令对象。
#[cfg(feature = "shell-tokio")]
pub fn apply_augmented_path_tokio_command(command: &mut tokio::process::Command) {
    if let Some(path) = effective_path_env() {
        command.env("PATH", path);
    }
}

/// 构建带有补全 PATH 的标准命令对象。
pub fn std_command(program: &str) -> std::process::Command {
    let mut command = std::process::Command::new(resolved_program_or_original(program));
    apply_augmented_path_std_command(&mut command);
    command
}

/// 构建带有补全 PATH 的异步命令对象。
#[cfg(feature = "shell-tokio")]
pub fn tokio_command(program: &str) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(resolved_program_or_original(program));
    apply_augmented_path_tokio_command(&mut command);
    command
}

/// 构建标准系统命令对象。
///
/// 适用于 `open`、`launchctl`、`kill` 等系统级命令；这类命令通常依赖系统默认
/// 环境，而不是用户 shell profile 中的 PATH 扩展。
pub fn std_system_command(program: &str) -> std::process::Command {
    std::process::Command::new(program)
}

/// 构建异步系统命令对象。
#[cfg(feature = "shell-tokio")]
pub fn tokio_system_command(program: &str) -> tokio::process::Command {
    tokio::process::Command::new(program)
}

/// 为标准 Git 命令附加统一环境变量。
pub fn apply_git_defaults_std_command(command: &mut std::process::Command) {
    for (key, value) in GIT_DEFAULT_ENV {
        command.env(key, value);
    }
}

/// 为异步 Git 命令附加统一环境变量。
#[cfg(feature = "shell-tokio")]
pub fn apply_git_defaults_tokio_command(command: &mut tokio::process::Command) {
    for (key, value) in GIT_DEFAULT_ENV {
        command.env(key, value);
    }
}

/// 构建标准 Git 命令对象。
pub fn git_std_command() -> std::process::Command {
    let mut command = std::process::Command::new(git_program());
    apply_augmented_path_std_command(&mut command);
    apply_git_defaults_std_command(&mut command);
    command
}

/// 构建异步 Git 命令对象。
#[cfg(feature = "shell-tokio")]
pub fn git_tokio_command() -> tokio::process::Command {
    let mut command = tokio::process::Command::new(git_program());
    apply_augmented_path_tokio_command(&mut command);
    apply_git_defaults_tokio_command(&mut command);
    command
}

/// 获取备用 Shell 路径
///
/// 当环境变量 `SHELL` 不可用或不合适时，根据操作系统选择默认 Shell。
fn fallback() -> String {
    #[cfg(windows)]
    {
        if let Some(p) = std::env::var("VIBEWINDOW_GIT_BASH_PATH").ok().as_ref() {
            let p = PathBuf::from(p);
            if p.is_file() {
                return p.to_string_lossy().to_string();
            }
        }
        if let Some(git) = which("git") {
            let bash =
                git.parent().and_then(|p| p.parent()).map(|p| p.join("bin").join("bash.exe"));
            if let Some(bash) = bash
                && bash.is_file()
            {
                return bash.to_string_lossy().to_string();
            }
        }
        return std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        "/bin/zsh".to_string()
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        which("bash")
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/bin/sh".to_string())
    }
}

/// 首选 Shell 路径
///
/// 基于环境变量 `SHELL` 或系统默认值确定的 Shell 路径。
pub static PREFERRED: Lazy<String> =
    Lazy::new(|| std::env::var("SHELL").unwrap_or_else(|_| fallback()));

/// 可接受的 Shell 路径
///
/// 与 `PREFERRED` 类似，但会过滤掉黑名单中的 Shell。
pub static ACCEPTABLE: Lazy<String> = Lazy::new(|| {
    if let Ok(s) = std::env::var("SHELL")
        && let Some(base) = basename(&s)
        && !BLACKLIST.contains(base.as_str())
    {
        return s;
    }
    fallback()
});

/// 终止进程树
///
/// 递归终止指定进程及其所有子进程。这是一个跨平台的实现。
pub fn kill_tree(proc: &mut Child, exited: Option<&dyn Fn() -> bool>) {
    let pid = proc.id();
    if pid == 0 || exited.is_some_and(|f| f()) {
        return;
    }

    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/pid", &pid.to_string(), "/f", "/t"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        return;
    }

    #[cfg(not(windows))]
    {
        let mut tried_group = false;
        if let Ok(status) =
            std::process::Command::new("kill").args(["-TERM", &format!("-{pid}")]).status()
        {
            tried_group = status.success();
        }

        if tried_group {
            std::thread::sleep(Duration::from_millis(SIGKILL_TIMEOUT_MS));
            if !exited.is_some_and(|f| f()) {
                let _ =
                    std::process::Command::new("kill").args(["-KILL", &format!("-{pid}")]).status();
            }
            return;
        }

        let _ = proc.kill();
        std::thread::sleep(Duration::from_millis(SIGKILL_TIMEOUT_MS));
        if !exited.is_some_and(|f| f()) {
            let _ = proc.kill();
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
