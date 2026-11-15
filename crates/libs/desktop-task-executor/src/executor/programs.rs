//! 任务执行器的 programs.rs 子模块。
//!
//! 该模块聚焦任务运行过程中的一个局部职责，供执行器入口组合调用。注释说明边界、错误传播和平台差异，避免调用方需要阅读完整执行链才能理解行为。

use super::*;

const OPENCODE_PACKAGE_SPEC: &str = "opencode-ai@latest";

/// 模块内部可见的 user_home_dir 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn user_home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

/// 模块内部可见的 opencode_binary_name 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn opencode_binary_name() -> &'static str {
    if cfg!(windows) { "opencode.exe" } else { "opencode" }
}

/// 模块内部可见的 claude_binary_name 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn claude_binary_name() -> &'static str {
    if cfg!(windows) { "claude.exe" } else { "claude" }
}

fn codex_binary_name() -> &'static str {
    if cfg!(windows) { "codex.exe" } else { "codex" }
}

fn normalize_program_name(program: &str) -> Option<String> {
    let file_name = Path::new(program).file_name()?.to_string_lossy().to_lowercase();
    Some(file_name)
}

/// 模块内部可见的 is_opencode_program 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn is_opencode_program(program: &str) -> bool {
    normalize_program_name(program).is_some_and(|name| name == opencode_binary_name())
}

/// 模块内部可见的 is_claude_program 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn is_claude_program(program: &str) -> bool {
    normalize_program_name(program).is_some_and(|name| name == claude_binary_name())
}

fn resolve_package_runner(program: &str) -> Option<String> {
    resolve_executable(program).map(|path| path.to_string_lossy().to_string())
}

pub fn select_opencode_program_and_prefix_args(
    opencode_bin: Option<String>,
    home: Option<&Path>,
    opencode_path: Option<PathBuf>,
    bunx_path: Option<String>,
    npx_path: Option<String>,
) -> (String, Vec<String>) {
    if let Some(path) = opencode_bin {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return (candidate.to_string_lossy().to_string(), Vec::new());
        }
    }
    if let Some(home) = home {
        let candidate = home.join(".opencode").join("bin").join(opencode_binary_name());
        if candidate.is_file() {
            return (candidate.to_string_lossy().to_string(), Vec::new());
        }
    }
    if let Some(found) = opencode_path {
        return (found.to_string_lossy().to_string(), Vec::new());
    }

    if let Some(bunx) = bunx_path {
        return (bunx, vec![OPENCODE_PACKAGE_SPEC.to_string()]);
    }

    if let Some(npx) = npx_path {
        return (npx, vec!["-y".to_string(), OPENCODE_PACKAGE_SPEC.to_string()]);
    }

    ("opencode".to_string(), Vec::new())
}

fn resolve_opencode_program_and_prefix_args() -> (String, Vec<String>) {
    let home = user_home_dir();
    select_opencode_program_and_prefix_args(
        shell_profile_env_var("OPENCODE_BIN"),
        home.as_deref(),
        resolve_executable(opencode_binary_name()),
        resolve_package_runner("bunx"),
        resolve_package_runner("npx"),
    )
}

/// 模块内部可见的 resolve_claude_program 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn select_claude_program(
    claude_bin: Option<String>,
    home: Option<&Path>,
    claude_path: Option<PathBuf>,
) -> String {
    if let Some(path) = claude_bin {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return candidate.to_string_lossy().to_string();
        }
    }
    if let Some(home) = home {
        let candidate = home.join(".claude").join("local").join(claude_binary_name());
        if candidate.is_file() {
            return candidate.to_string_lossy().to_string();
        }
    }
    if let Some(found) = claude_path {
        return found.to_string_lossy().to_string();
    }
    claude_binary_name().to_string()
}

pub fn resolve_claude_program() -> String {
    let home = user_home_dir();
    select_claude_program(
        shell_profile_env_var("CLAUDE_BIN"),
        home.as_deref(),
        resolve_executable(claude_binary_name()),
    )
}

fn resolve_codex_program() -> String {
    if let Some(path) = shell_profile_env_var("CODEX_BIN") {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return candidate.to_string_lossy().to_string();
        }
    }
    if let Some(found) = resolve_executable(codex_binary_name()) {
        return found.to_string_lossy().to_string();
    }
    codex_binary_name().to_string()
}

/// 模块内部可见的 spawn_executor_child 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn spawn_executor_child(cmd: &ExecutorCommand) -> Result<std::process::Child, String> {
    let program = resolve_executable(&cmd.program).unwrap_or_else(|| PathBuf::from(&cmd.program));
    let mut command = std_command(program.to_string_lossy().as_ref());
    let stdin = if cmd.stdin_content.is_some() { Stdio::piped() } else { Stdio::null() };
    command
        .args(&cmd.args)
        .current_dir(&cmd.cwd)
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    command.spawn().map_err(|e| format!("Failed to spawn {}: {}", program.display(), e))
}

/// 公开的 ExecutorCommand 结构体，承载该模块边界内传递的结构化状态。
#[derive(Debug, Clone)]
pub struct ExecutorCommand {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub stdin_content: Option<String>,
}

impl ExecutorCommand {
    pub fn for_opencode_resolved(
        project_path: &str,
        model: &str,
        prompt: &str,
        program: String,
        mut args: Vec<String>,
    ) -> Self {
        args.extend([
            "run".to_string(),
            format!("--dir={}", project_path),
            "--format=json".to_string(),
            "--thinking".to_string(),
        ]);
        if model != "auto" {
            args.push(format!("--model={}", model));
        }
        if !prompt.trim().is_empty() {
            args.push(prompt.to_string());
        }
        Self { program, args, cwd: project_path.to_string(), stdin_content: None }
    }

    /// 公开的 for_opencode 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn for_opencode(project_path: &str, model: &str, prompt: &str) -> Self {
        let (program, args) = resolve_opencode_program_and_prefix_args();
        Self::for_opencode_resolved(project_path, model, prompt, program, args)
    }

    /// 公开的 for_claude 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn for_claude(project_path: &str, model: &str, prompt: &str) -> Self {
        let mut args = vec![
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--print".to_string(),
            "--verbose".to_string(),
            "--thinking".to_string(),
            "enabled".to_string(),
            "--dangerously-skip-permissions".to_string(),
        ];

        if let Some(claude_model) = claude_model_alias(model) {
            args.push("--model".to_string());
            args.push(claude_model.to_string());
        } else {
            args.push(format!("--model={}", CLAUDE_DEFAULT_MODEL_ALIAS));
        }

        args.extend(["--add-dir".to_string(), project_path.to_string()]);

        Self {
            program: resolve_claude_program(),
            args,
            cwd: project_path.to_string(),
            stdin_content: Some(prompt.to_string()),
        }
    }

    /// 公开的 for_codex 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn for_codex(project_path: &str, model: &str, prompt: &str) -> Self {
        let mut args = vec!["exec".to_string(), "--skip-git-repo-check".to_string()];
        if model != "auto" {
            args.push("--model".to_string());
            args.push(model.to_string());
        }
        args.push(prompt.to_string());
        Self {
            program: resolve_codex_program(),
            args,
            cwd: project_path.to_string(),
            stdin_content: None,
        }
    }
}

/// 公开的 build_executor_command 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn build_executor_command(
    backend: TaskExecutorBackend,
    project_path: &str,
    model: &str,
    prompt: &str,
) -> ExecutorCommand {
    match backend {
        TaskExecutorBackend::OpenCode => ExecutorCommand::for_opencode(project_path, model, prompt),
        TaskExecutorBackend::Claude => ExecutorCommand::for_claude(project_path, model, prompt),
        TaskExecutorBackend::Internal => ExecutorCommand::for_opencode(project_path, model, prompt),
        TaskExecutorBackend::Codex => ExecutorCommand::for_codex(project_path, model, prompt),
    }
}

#[cfg(test)]
#[path = "programs_tests.rs"]
mod programs_tests;
