//! 格式化器可用性探测。
//!
//! 该模块根据二进制是否存在、项目配置文件、依赖声明和实验开关判断某个
//! 内置格式化器是否适用于当前工作区，并把结果缓存在实例状态中。

use super::state::{EnabledCheck, FormatterInfo, State};
use runtime_flags as flag;
use crate::host as instance;
use shell_command::tokio_command;
use crate::host as filesystem;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

/// 判断格式化器在当前实例中是否可用。
///
/// # 参数
///
/// - `state`: 当前项目实例的格式化状态，包含缓存和内置格式化器表。
/// - `item`: 需要探测的格式化器配置。
///
/// # 返回值
///
/// 返回格式化器是否可用。结果会按格式化器名称写入缓存，避免频繁扫描
/// PATH、项目文件或重复启动外部命令。
pub(super) async fn is_enabled(state: &Arc<State>, item: &FormatterInfo) -> bool {
    let cached = state.enabled.lock().ok().and_then(|m| m.get(&item.name).copied());
    if let Some(v) = cached {
        return v;
    }

    let computed = item.enabled().await;
    if let Ok(mut lock) = state.enabled.lock() {
        lock.insert(item.name.clone(), computed);
    }
    computed
}

impl FormatterInfo {
    async fn enabled(&self) -> bool {
        match self.enabled {
            EnabledCheck::Always => true,
            EnabledCheck::Which(program) => which(program).is_some(),
            EnabledCheck::FileUpAny(targets) => {
                let start = instance::directory();
                if start.trim().is_empty() {
                    return false;
                }
                let stop = instance::worktree();
                for target in targets {
                    let found = filesystem::find_up(
                        target,
                        &start,
                        if stop.trim().is_empty() { None } else { Some(&stop) },
                    )
                    .await;
                    if !found.is_empty() {
                        return true;
                    }
                }
                false
            }
            EnabledCheck::Prettier => prettier_enabled().await,
            EnabledCheck::Oxfmt => oxfmt_enabled().await,
            EnabledCheck::Biome => biome_enabled().await,
            EnabledCheck::ClangFormat => clang_format_enabled().await,
            EnabledCheck::Ruff => ruff_enabled().await,
            EnabledCheck::UvFormat => uvformat_enabled().await,
            EnabledCheck::Pint => pint_enabled().await,
            EnabledCheck::Ocamlformat => ocamlformat_enabled().await,
            EnabledCheck::RLangAir => rlang_air_enabled().await,
        }
    }
}

pub(super) fn which(program: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let path = dir.join(program);
        if path.is_file() {
            return Some(path);
        }
        #[cfg(windows)]
        {
            let path = dir.join(format!("{program}.exe"));
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

pub(super) async fn prettier_enabled() -> bool {
    if which("bun").is_none() {
        return false;
    }

    let start = instance::directory();
    if start.trim().is_empty() {
        return false;
    }
    let stop = instance::worktree();

    let items = filesystem::find_up(
        "package.json",
        &start,
        if stop.trim().is_empty() { None } else { Some(&stop) },
    )
    .await;

    for item in items {
        if let Ok(text) = tokio::fs::read_to_string(&item).await {
            if let Ok(json) = serde_json::from_str::<Value>(&text) {
                if has_dep(&json, "prettier") {
                    return true;
                }
            }
        }
    }
    false
}

pub(super) async fn oxfmt_enabled() -> bool {
    if !*flag::VIBEWINDOW_EXPERIMENTAL_OXFMT {
        return false;
    }
    if which("bun").is_none() {
        return false;
    }

    let start = instance::directory();
    if start.trim().is_empty() {
        return false;
    }
    let stop = instance::worktree();

    let items = filesystem::find_up(
        "package.json",
        &start,
        if stop.trim().is_empty() { None } else { Some(&stop) },
    )
    .await;

    for item in items {
        if let Ok(text) = tokio::fs::read_to_string(&item).await {
            if let Ok(json) = serde_json::from_str::<Value>(&text) {
                if has_dep(&json, "oxfmt") {
                    return true;
                }
            }
        }
    }
    false
}

pub(super) async fn biome_enabled() -> bool {
    if which("bun").is_none() {
        return false;
    }

    let start = instance::directory();
    if start.trim().is_empty() {
        return false;
    }
    let stop = instance::worktree();

    for config in ["biome.json", "biome.jsonc"] {
        let found = filesystem::find_up(
            config,
            &start,
            if stop.trim().is_empty() { None } else { Some(&stop) },
        )
        .await;
        if !found.is_empty() {
            return true;
        }
    }
    false
}

pub(super) async fn clang_format_enabled() -> bool {
    if which("clang-format").is_none() {
        return false;
    }

    let start = instance::directory();
    if start.trim().is_empty() {
        return false;
    }
    let stop = instance::worktree();

    let found = filesystem::find_up(
        ".clang-format",
        &start,
        if stop.trim().is_empty() { None } else { Some(&stop) },
    )
    .await;
    !found.is_empty()
}

pub(super) async fn ocamlformat_enabled() -> bool {
    if which("ocamlformat").is_none() {
        return false;
    }

    let start = instance::directory();
    if start.trim().is_empty() {
        return false;
    }
    let stop = instance::worktree();

    let found = filesystem::find_up(
        ".ocamlformat",
        &start,
        if stop.trim().is_empty() { None } else { Some(&stop) },
    )
    .await;
    !found.is_empty()
}

pub(super) async fn ruff_enabled() -> bool {
    if which("ruff").is_none() {
        return false;
    }

    let start = instance::directory();
    if start.trim().is_empty() {
        return false;
    }
    let stop = instance::worktree();

    for config in ["pyproject.toml", "ruff.toml", ".ruff.toml"] {
        let found = filesystem::find_up(
            config,
            &start,
            if stop.trim().is_empty() { None } else { Some(&stop) },
        )
        .await;
        if found.is_empty() {
            continue;
        }

        if config == "pyproject.toml" {
            if let Ok(content) = tokio::fs::read_to_string(&found[0]).await {
                if content.contains("[tool.ruff]") {
                    return true;
                }
            }
        } else {
            return true;
        }
    }

    for dep in ["requirements.txt", "pyproject.toml", "Pipfile"] {
        let found = filesystem::find_up(
            dep,
            &start,
            if stop.trim().is_empty() { None } else { Some(&stop) },
        )
        .await;
        if found.is_empty() {
            continue;
        }
        if let Ok(content) = tokio::fs::read_to_string(&found[0]).await {
            if content.contains("ruff") {
                return true;
            }
        }
    }
    false
}

pub(super) async fn uvformat_enabled() -> bool {
    if ruff_enabled().await {
        return false;
    }
    if which("uv").is_none() {
        return false;
    }

    let status = tokio_command("uv")
        .args(["format", "--help"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await;
    status.is_ok_and(|s| s.success())
}

pub(super) async fn pint_enabled() -> bool {
    let start = instance::directory();
    if start.trim().is_empty() {
        return false;
    }
    let stop = instance::worktree();

    let items = filesystem::find_up(
        "composer.json",
        &start,
        if stop.trim().is_empty() { None } else { Some(&stop) },
    )
    .await;

    for item in items {
        if let Ok(text) = tokio::fs::read_to_string(&item).await {
            if let Ok(json) = serde_json::from_str::<Value>(&text) {
                if json
                    .get("require")
                    .and_then(|v| v.as_object())
                    .is_some_and(|m| m.contains_key("laravel/pint"))
                {
                    return true;
                }
                if json
                    .get("require-dev")
                    .and_then(|v| v.as_object())
                    .is_some_and(|m| m.contains_key("laravel/pint"))
                {
                    return true;
                }
            }
        }
    }
    false
}

pub(super) async fn rlang_air_enabled() -> bool {
    if which("air").is_none() {
        return false;
    }

    let out = tokio_command("air")
        .arg("--help")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await;

    let Ok(out) = out else {
        return false;
    };
    if !out.status.success() {
        return false;
    }

    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let first_line = text.lines().next().unwrap_or_default();
    // air 名称较短，环境中可能存在同名非 R 工具；检查 help 首行可降低误判。
    first_line.contains("R language") && first_line.contains("formatter")
}

pub(super) fn has_dep(json: &Value, name: &str) -> bool {
    json.get("dependencies").and_then(|v| v.as_object()).is_some_and(|m| m.contains_key(name))
        || json
            .get("devDependencies")
            .and_then(|v| v.as_object())
            .is_some_and(|m| m.contains_key(name))
}
