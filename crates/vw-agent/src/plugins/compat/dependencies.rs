//! Read-only package/runtime probes. No process launch, import, network or package mutation.
use super::mcp_legacy::PluginMcpDeclaration;
use agent_client_protocol::McpServer;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fs,
    path::{Component, Path},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyState {
    Present,
    MissingHost,
    MissingPackage,
    Unverified,
    ScopeExcluded,
}
impl DependencyState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::MissingHost => "missing_host",
            Self::MissingPackage => "missing_package",
            Self::Unverified => "unverified",
            Self::ScopeExcluded => "scope_excluded",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependencyCheck {
    pub component: String,
    pub dependency: String,
    pub state: DependencyState,
    pub detail: String,
}
fn check(
    component: &str,
    dependency: &str,
    state: DependencyState,
    detail: &str,
) -> PluginDependencyCheck {
    PluginDependencyCheck {
        component: component.into(),
        dependency: dependency.into(),
        state,
        detail: detail.into(),
    }
}
fn package_file(
    root: &Path,
    raw: &str,
    component: &str,
    executable: bool,
) -> Result<PluginDependencyCheck> {
    let relative = raw.strip_prefix("${PLUGIN_ROOT}/").unwrap_or(raw).trim_start_matches("./");
    let path = Path::new(relative);
    if path.is_absolute()
        || relative.contains(['\\', '\0'])
        || path.components().any(|p| !matches!(p, Component::Normal(_)))
        || relative.split('/').any(|p| p == "..")
    {
        bail!("unsafe plugin dependency path");
    }
    let target = root.join(path);
    let mut ancestor = root.to_path_buf();
    for part in path.components() {
        ancestor.push(part.as_os_str());
        match fs::symlink_metadata(&ancestor) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                bail!("plugin dependency must not follow symlinks")
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(check(
                    component,
                    relative,
                    DependencyState::MissingPackage,
                    "包内资源缺失；请核对原始插件包，勿由宿主安装时改包",
                ));
            }
            Err(error) => return Err(error.into()),
        }
    }
    let metadata = fs::metadata(&target)?;
    if !target.canonicalize()?.starts_with(root) {
        bail!("plugin dependency escapes package");
    }
    let mut available = metadata.is_file();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if executable {
            available &= metadata.permissions().mode() & 0o111 != 0;
        }
    }
    #[cfg(not(unix))]
    {
        if executable {
            available &= which::which(&target).is_ok();
        }
    }
    Ok(check(
        component,
        relative,
        if available { DependencyState::Present } else { DependencyState::MissingPackage },
        if available {
            "包内文件存在；未执行其内容"
        } else {
            "包内资源不是可用文件或缺少执行权限；请核对发行包"
        },
    ))
}
fn host_command(
    component: &str,
    command: &str,
    path: Option<OsString>,
    cwd: &Path,
) -> PluginDependencyCheck {
    let found = which::which_in(command, path, cwd).is_ok();
    check(
        component,
        command,
        if found { DependencyState::Present } else { DependencyState::MissingHost },
        if found {
            "可执行文件存在；未启动进程"
        } else {
            "宿主找不到可执行文件；请在该宿主安装或配置正确 PATH 后重试"
        },
    )
}
fn stdio(
    root: &Path,
    name: &str,
    command: &str,
    args: &[String],
    portable: bool,
    cwd: Option<&str>,
    path: Option<OsString>,
) -> Result<Vec<PluginDependencyCheck>> {
    let known_cwd = (portable && cwd.is_none())
        || cwd.is_some_and(|cwd| {
            cwd == "." || cwd == "./" || cwd == "${PLUGIN_ROOT}" || Path::new(cwd) == root
        });
    let mut checks = Vec::new();
    if command.starts_with("./") && portable {
        checks.push(package_file(root, command, name, true)?);
    } else if command.contains("${")
        || (!portable && !Path::new(command).is_absolute() && command.contains(['/', '\\']))
        || path.as_ref().is_some_and(|p| {
            p.to_string_lossy().contains("${")
                || (!known_cwd && std::env::split_paths(p).any(|p| !p.is_absolute()))
        })
    {
        checks.push(check(
            name,
            "command",
            DependencyState::Unverified,
            "命令包含尚未解析的变量；不能认定可执行",
        ));
    } else {
        checks.push(host_command(name, command, path, root));
    }
    let binary = Path::new(command).file_stem().and_then(|p| p.to_str()).unwrap_or(command);
    let interpreter =
        matches!(binary, "node" | "nodejs" | "python" | "python2" | "python3" | "sh" | "bash")
            || binary.starts_with("python3.");
    if !interpreter {
        if !args.is_empty() {
            checks.push(check(
                name,
                "arguments",
                DependencyState::Unverified,
                "仅确认可执行文件；参数声明的动态依赖仍需受控验证",
            ));
        }
        return Ok(checks);
    }
    let mut arguments = args.iter();
    while let Some(arg) = arguments.next() {
        if matches!(arg.as_str(), "-u" | "-B" | "--no-warnings" | "--enable-source-maps") {
            continue;
        }
        if arg == "-m" && binary.starts_with("python") {
            let module = arguments.next().context("Python -m requires a module")?;
            if !module
                .split('.')
                .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
            {
                bail!("invalid Python dependency module");
            }
            let module_path = module.replace('.', "/");
            let candidates = [format!("{module_path}.py"), format!("{module_path}/__main__.py")];
            let observations = candidates
                .iter()
                .map(|p| package_file(root, p, name, false))
                .collect::<Result<Vec<_>>>()?;
            if known_cwd
                && let Some(found) =
                    observations.iter().find(|p| p.state == DependencyState::Present)
            {
                checks.push(found.clone());
            } else {
                checks.push(check(
                    name,
                    module,
                    DependencyState::Unverified,
                    "模块未确认为包内入口；未导入外部模块，请在受控环境核对模块依赖",
                ));
            }
            break;
        }
        if arg.starts_with('-') {
            checks.push(check(
                name,
                "arguments",
                DependencyState::Unverified,
                "内联代码或复杂启动参数未执行，依赖仍待验证",
            ));
            break;
        }
        if arg.contains("${") && (!portable || !arg.starts_with("${PLUGIN_ROOT}/")) {
            checks.push(check(
                name,
                "arguments",
                DependencyState::Unverified,
                "运行时变量或包外数据依赖尚待验证",
            ));
            break;
        }
        if arg.starts_with("${PLUGIN_ROOT}/") || known_cwd {
            checks.push(package_file(root, arg, name, false)?);
        } else {
            checks.push(check(
                name,
                "working_directory",
                DependencyState::Unverified,
                "旧版相对路径缺少显式包工作目录；不能将宿主目录中的资源误判为包内资源",
            ));
        }
        break;
    }
    Ok(checks)
}
pub fn inspect_plugin_dependencies(root: &Path) -> Result<Vec<PluginDependencyCheck>> {
    let manifest = crate::plugins::load_plugin_manifest(root)?;
    let root = &manifest.root;
    let (outcome, _) = crate::plugins::mcp::load_plugin_mcp_declarations(&manifest)?;
    if !outcome.errors.is_empty() {
        bail!("plugin MCP contains invalid dependency declarations");
    }
    let mut checks = Vec::new();
    for declaration in outcome.servers {
        match declaration {
            PluginMcpDeclaration::Legacy { name, config }
            | PluginMcpDeclaration::Portable { name, config } => {
                if config.get("enabled") == Some(&serde_json::Value::Bool(false)) {
                    continue;
                }
                let portable = manifest.manifest_path == root.join("plugin.json");
                if let Some(command) = config.get("command").and_then(|v| v.as_str()) {
                    let args = config
                        .get("args")
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .map(|v| {
                                    v.as_str().map(str::to_owned).context("invalid MCP argument")
                                })
                                .collect::<Result<Vec<_>>>()
                        })
                        .transpose()?
                        .unwrap_or_default();
                    let path = config
                        .get("env")
                        .and_then(|v| v.as_object())
                        .and_then(|env| {
                            env.iter()
                                .find(|(name, _)| {
                                    super::mcp_portable::env_names_match(name, "PATH")
                                })
                                .map(|(_, value)| value)
                        })
                        .and_then(|v| v.as_str())
                        .map(|p| {
                            OsString::from(p.replace("${PLUGIN_ROOT}", &root.to_string_lossy()))
                        })
                        .or_else(|| std::env::var_os("PATH"));
                    checks.extend(stdio(
                        root,
                        &name,
                        command,
                        &args,
                        portable,
                        config.get("cwd").and_then(|v| v.as_str()),
                        path,
                    )?);
                } else {
                    checks.push(check(
                        &name,
                        "公开 MCP 服务",
                        DependencyState::Unverified,
                        "远程服务和鉴权未联网探测；不能由声明推断运行可用",
                    ));
                }
            }
            PluginMcpDeclaration::Acp(server) => match server {
                McpServer::Stdio(server) => {
                    checks.extend(stdio(
                        root,
                        &server.name,
                        &server.command.to_string_lossy(),
                        &server.args,
                        false,
                        None,
                        std::env::var_os("PATH"),
                    )?);
                }
                _ => checks.push(check(
                    "MCP",
                    "公开 MCP 服务",
                    DependencyState::Unverified,
                    "远程连接和鉴权仍需验证",
                )),
            },
        }
    }
    for skill_root in super::skill_roots::skill_roots(&manifest)? {
        let (files, errors) = super::skill_roots::skill_files(
            &skill_root,
            super::skill_roots::package_format(&manifest),
        );
        if !errors.is_empty() {
            bail!("unsafe plugin skill dependency inventory");
        }
        for file in files {
            let metadata =
                super::agents::load_metadata(root, file.parent().context("missing skill parent")?);
            if !metadata.diagnostics.is_empty() {
                checks.push(check(
                    "skill",
                    "agent metadata",
                    DependencyState::Unverified,
                    "技能依赖元数据存在诊断；请核对声明",
                ));
            }
            if let Some(metadata) = metadata.metadata {
                for dependency in metadata.dependencies {
                    let detail = if dependency.r#type == "mcp" {
                        "公开 MCP 工具依赖需要独立配置和连接验证；声明不授予权限"
                    } else {
                        "宿主工具映射或服务依赖尚待验证；不能据工具名认定兼容"
                    };
                    checks.push(check(
                        "skill",
                        &dependency.value,
                        DependencyState::Unverified,
                        detail,
                    ));
                }
            }
        }
    }
    if manifest.apps.is_some() {
        checks.push(check(
            "apps",
            "托管连接器",
            DependencyState::ScopeExcluded,
            "不提供托管连接器；独立技能、公开 MCP 和公开应用界面另行验证",
        ));
    }
    Ok(checks)
}
/// Preflight the actual resolved executable before the MCP inspection path can launch it.
pub(crate) fn missing_mcp_executable(server: &McpServer) -> Result<Option<PluginDependencyCheck>> {
    let McpServer::Stdio(server) = server else {
        return Ok(None);
    };
    let cwd = super::mcp_environment::runtime_cwd(server)?.unwrap_or(std::env::current_dir()?);
    let path = server
        .env
        .iter()
        .find(|e| super::mcp_portable::env_names_match(&e.name, "PATH"))
        .map(|e| OsString::from(&e.value))
        .or_else(|| std::env::var_os("PATH"));
    let diagnostic = host_command(&server.name, &server.command.to_string_lossy(), path, &cwd);
    Ok((diagnostic.state == DependencyState::MissingHost).then_some(diagnostic))
}
