//! ACP 代理子进程生命周期管理。
//!
//! 本模块封装代理进程的启动、进程组清理和 stderr 收集，让 actor 模块只关心
//! 运行时连接是否可用以及何时重启。

use super::*;

impl AcpClient {
    pub(super) fn managed_cortex(&self) -> bool {
        self.agent_name == "cortex-code"
            && self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
    }

    pub(super) fn managed_aionrs(&self) -> bool {
        self.agent_name == "aionrs" && self.config.env.contains_key("SHIJU_AIONRS_SANDBOX")
            && self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
    }

    pub(super) fn managed_vtcode(&self) -> bool {
        self.agent_name == "vtcode" && self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
    }

    pub(super) fn managed_corust(&self) -> bool {
        self.agent_name == "corust-agent" && self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
    }

    fn managed_restricted_pi(&self) -> bool {
        self.agent_name == "pi"
            && self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
            && self.config.env.get("VW_PI_RESTRICTED").is_some_and(|value| value == "1")
    }

    pub(super) fn sandboxed_native_io(&self) -> bool {
        self.droid_sandbox.is_some() || self.managed_restricted_pi() || self.auggie_sandbox.is_some() || self.kilocode_sandbox.is_some() || self.hermes_sandbox.is_some() || self.opencode_sandbox.is_some() || self.openclaw_sandbox.is_some() || self.managed_vtcode() || self.managed_aionrs() || self.managed_cortex()
    }

    pub(super) fn managed_workspace_io(&self) -> bool {
        matches!(self.agent_name.as_str(), "mimo-code" | "omp")
            && self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
    }

    /// Reject desktop routes whose native tools can execute outside client approval.
    pub fn ensure_desktop_native_access(
        agent_name: &str,
        unrestricted: bool,
    ) -> Result<(), AcpError> {
        let backend = crate::agent_registry::desktop_permission_agent(agent_name);
        if !unrestricted
            && let Some(
                backend @ ("auggie" | "droid" | "hermes" | "kilocode" | "openclaw" | "opencode"
                | "pi"),
            ) = backend
        {
            return Err(AcpError::Initialize(format!(
                "{backend} approval mode cannot constrain native tools; a current full access grant is required"
            )));
        }
        Ok(())
    }

    /// 启动 ACP 代理子进程并返回进程句柄。
    ///
    /// 该函数会合并配置环境变量和认证环境变量，创建可管控的 stdin/stdout/stderr
    /// 管道。命令为空或进程启动失败时返回 [`AcpError`]。在 Unix 上，子进程会被
    /// 放入独立进程组，便于后续关闭时收敛整组进程。
    pub(crate) fn spawn_child(&self) -> Result<ProcessHandles, AcpError> {
        self.spawn_child_with_stderr(false)
    }

    pub(super) fn spawn_child_with_stderr(
        &self,
        discard_stderr: bool,
    ) -> Result<ProcessHandles, AcpError> {
        self.spawn_child_at(discard_stderr, None)
    }

    pub(super) fn spawn_child_at(
        &self,
        discard_stderr: bool,
        directory: Option<&Path>,
    ) -> Result<ProcessHandles, AcpError> {
        if self.agent_name.starts_with("vw-session-model:")
            || self.config.command.starts_with("vw-session-model:")
        {
            return Err(AcpError::Initialize(
                "managed model launch must be reissued by its session owner".into(),
            ));
        }
        let discard_stderr =
            discard_stderr || self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE");
        if self.config.command.trim().is_empty() {
            return Err(AcpError::EmptyCommand);
        }

        if matches!(self.agent_name.as_str(), "deepagents" | "sigit" | "amp-acp") {
            // Local discovery, npx fallback and explicit overrides share this boundary.
            // Never execute the supplier's unpatched permission/file fallbacks.
            crate::verify_reviewed_agent_executable(&self.agent_name, self.config.command.trim())
                .map_err(AcpError::Initialize)?;
        }
        let args = self.launch_args()?;
        if self.agent_name == "sigit" && args != ["--acp"] {
            return Err(AcpError::Initialize(
                "siGit reviewed runtime requires exactly --acp".into(),
            ));
        }
        if self.agent_name == "amp-acp" && !args.is_empty() {
            return Err(AcpError::Initialize(
                "Amp reviewed runtime requires empty arguments".into(),
            ));
        }
        let mut env = self.config.env.clone();
        if self.managed_restricted_pi() {
            env.retain(|key, _| matches!(key.as_str(),
                "VW_ACP_LAUNCH_REFERENCE" | "VW_PI_RESTRICTED" | "VW_PI_RELAY_TOKEN"
                | "PI_ACP_PI_COMMAND" | "PI_ACP_STATE_DIR" | "PI_CODING_AGENT_DIR"
                | "PI_OFFLINE" | "PI_SKIP_VERSION_CHECK" | "SHIJU_PI_SKILL_ROOT"
            ));
            let home = env.get("PI_CODING_AGENT_DIR").cloned().ok_or_else(|| {
                AcpError::Initialize("Missing restricted Pi private home".into())
            })?;
            env.insert("HOME".into(), home);
        }
        if self.agent_name == "amp-acp" {
            if env.contains_key("VW_ACP_LAUNCH_REFERENCE") {
                env.entry("SHIJU_AMP_SKILL_VIEW".into()).or_insert_with(|| "null".into());
            } else {
                env.insert("SHIJU_AMP_SKILL_VIEW".into(), "null".into());
            }
            for key in [
                "NODE_OPTIONS",
                "NODE_PATH",
                "BASH_ENV",
                "ENV",
                "LD_PRELOAD",
                "DYLD_INSERT_LIBRARIES",
                "DYLD_LIBRARY_PATH",
            ] {
                env.insert(key.into(), String::new());
            }
        }
        if self.agent_name == "copilot" {
            // An inherited CLI grant must not override the current host policy.
            // Explicit authorized flags remain supported; never inject a blanket grant.
            env.remove("COPILOT_ALLOW_ALL");
        }
        if self.agent_name == "qwen"
            && crate::access_policy::qwen::verified_entry(&self.config.command, &self.config.args)
        {
            let entry = Path::new(&self.config.command).canonicalize().map_err(AcpError::Spawn)?;
            // The npm wrapper can redirect to an independently updated cli.js.
            // Pin this process to the exact package that will pass identity checks.
            env.insert(
                "QWEN_CODE_MANAGED_NPM_PIN".into(),
                json!({
                    "bootstrap": entry,
                    "version": null,
                    "updateRoot": entry.parent().unwrap()
                })
                .to_string(),
            );
        }
        let path = Path::new(self.config.command.trim());
        // The connectivity probe receives only a command, without the agent key.
        let glm_managed_path = path.ends_with("node_modules/glm-acp-agent/dist/index.js")
            || path.ends_with("node_modules/.bin/glm-acp-agent")
            || path.ends_with("node_modules/.bin/glm-acp-agent.cmd");
        let glm_entry = (self.agent_name == "glm-acp-agent" || glm_managed_path)
            .then(|| {
                let entry = path.canonicalize().map_err(AcpError::Spawn)?;
                let path = entry.to_str().ok_or_else(|| {
                    AcpError::Initialize("Managed GLM entry requires a Unicode path".into())
                })?;
                crate::verify_reviewed_agent_executable("glm-acp-agent", path)
                    .map_err(AcpError::Initialize)?;
                Ok(entry)
            })
            .filter(|review| {
                review.is_ok()
                    || glm_managed_path
                    || env.contains_key("VW_ACP_LAUNCH_REFERENCE")
                    || (cfg!(windows) && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("js")))
            });
        let managed_vtcode = self.managed_vtcode();
        let managed_grok =
            self.agent_name == "grok" && env.contains_key("VW_ACP_LAUNCH_REFERENCE");
        let managed_private_provider = self.openclaw_sandbox.is_some() || self.managed_restricted_pi() || managed_grok || managed_vtcode || self.managed_workspace_io() || self.managed_aionrs()
            || self.managed_corust() || self.managed_cortex()
            || (matches!(
                self.agent_name.as_str(),
                "deepagents" | "dimcode" | "junie" | "sigit" | "stakpak" | "poolside"
            ) && env.contains_key("VW_ACP_LAUNCH_REFERENCE"));
        let managed_claude = managed_claude_settings(&self.config).is_some();
        if self.managed_cortex() {
            crate::verify_reviewed_agent_executable("cortex-code", self.config.command.trim())
                .map_err(AcpError::Initialize)?;
        }
        let managed_auggie =
            self.agent_name == "auggie" && env.contains_key("VW_ACP_LAUNCH_REFERENCE");
        let managed_droid =
            self.agent_name == "droid" && env.contains_key("VW_ACP_LAUNCH_REFERENCE");
        if managed_droid {
            crate::verify_reviewed_agent_executable("droid", self.config.command.trim())
                .map_err(AcpError::Initialize)?;
        }
        let managed_cursor =
            self.agent_name == "cursor" && env.contains_key("VW_ACP_LAUNCH_REFERENCE");
        if managed_cursor {
            crate::verify_reviewed_agent_executable("cursor", self.config.command.trim())
                .map_err(AcpError::Initialize)?;
        }
        if managed_auggie {
            crate::verify_reviewed_agent_executable("auggie", self.config.command.trim())
                .map_err(AcpError::Initialize)?;
        }
        if managed_claude && self.agent_name == "claude" {
            if !args.is_empty() {
                return Err(AcpError::Initialize(
                    "Claude managed launch requires verified ACP adapter arguments".into(),
                ));
            }
            let command = self.config.command.trim();
            if env.get("VW_ACP_CLAUDE_NATIVE_MESSAGES").is_some_and(|value| value == "1") {
                crate::verify_reviewed_agent_executable("claude", command)
                    .map_err(AcpError::Initialize)?;
            } else if !Path::new(command).is_absolute()
                || !crate::access_policy::claude::verified_entry(command, &args)
            {
                // The effort runtime sends native output_config even with thinking disabled.
                return Err(AcpError::Initialize(
                    "Claude Chat conversion requires the reviewed 0.26.0 adapter".into(),
                ));
            }
        }
        if self.kilocode_sandbox.is_none() && self.hermes_sandbox.is_none() && self.opencode_sandbox.is_none() && !managed_claude && !managed_auggie && !managed_cursor && !managed_droid && !managed_private_provider {
            for (key, value) in build_agent_environment(&self.auth_credentials) {
                insert_auth_env_if_missing(&mut env, key, value);
            }
        }

        let mut cmd = if let Some(sandbox) = &self.kilocode_sandbox {
            let workspace_allowed = self.desktop_workspace.as_ref().is_some_and(|workspace| {
                sandbox.workspace.starts_with(workspace)
            });
            if self.agent_name != "kilocode" || !workspace_allowed || self.full_access
                || self.permission_mode != PermissionMode::ApproveReads
                || !self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
            {
                return Err(AcpError::Initialize("Invalid restricted Kilocode launch".into()));
            }
            sandbox.command(self.config.command.trim(), &args, &env)?
        } else if let Some(sandbox) = &self.openclaw_sandbox {
            let workspace_allowed = self.desktop_workspace.as_ref()
                .and_then(|workspace| workspace.canonicalize().ok())
                .is_some_and(|workspace| sandbox.workspace.starts_with(workspace));
            if self.agent_name != "openclaw" || !workspace_allowed || self.full_access
                || self.permission_mode != PermissionMode::ApproveReads
                || self.non_interactive_permissions.is_some()
                || !self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
            {
                return Err(AcpError::Initialize("Invalid restricted OpenClaw launch".into()));
            }
            sandbox.command(self.config.command.trim(), &args, &env)?
        } else if let Some(sandbox) = &self.opencode_sandbox {
            let workspace_allowed = self.desktop_workspace.as_ref().is_some_and(|workspace| {
                sandbox.workspace.starts_with(workspace)
            });
            if self.agent_name != "opencode" || !workspace_allowed || self.full_access
                || self.permission_mode != PermissionMode::ApproveReads
                || !self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
            {
                return Err(AcpError::Initialize("Invalid restricted OpenCode launch".into()));
            }
            sandbox.command(self.config.command.trim(), &args, &env)?
        } else if let Some(sandbox) = &self.auggie_sandbox {
            let workspace_allowed = self.desktop_workspace.as_ref()
                .and_then(|workspace| workspace.canonicalize().ok())
                .is_some_and(|workspace| sandbox.workspace.starts_with(workspace));
            if !workspace_allowed {
                return Err(AcpError::Initialize("Restricted Auggie workspace is outside desktop access".into()));
            }
            if env.keys().any(|name| !matches!(name.as_str(),
                "VW_ACP_LAUNCH_REFERENCE" | "AUGMENT_DISABLE_AUTO_UPDATE" | "AUGMENT_SESSION_AUTH"
                | "SHIJU_AUGGIE_MODEL_CONFIG" | "SHIJU_AUGGIE_SKILL_VIEW" | "SHIJU_AUGGIE_MCP_TOOLS"
            )) {
                return Err(AcpError::Initialize("Restricted Auggie requires its generated launch environment".into()));
            }
            env.insert("SHIJU_AUGGIE_WORKSPACE".into(), sandbox.workspace.to_string_lossy().into_owned());
            env.insert("SHIJU_AUGGIE_RESTRICTED".into(), "1".into());
            env.insert("AUGMENT_DISABLE_AUTO_UPDATE".into(), "1".into());
            env.insert("HOME".into(), sandbox.home.to_string_lossy().into_owned());
            env.insert("TMPDIR".into(), sandbox.home.to_string_lossy().into_owned());
            env.insert("NO_PROXY".into(), "*".into());
            sandbox.command(self.config.command.trim())?
        } else if let Some(sandbox) = &self.droid_sandbox {
            let allowed = self.desktop_workspace.as_ref()
                .and_then(|workspace| workspace.canonicalize().ok())
                .is_some_and(|workspace| sandbox.workspace.starts_with(workspace));
            if !allowed {
                return Err(AcpError::Initialize("Restricted Droid workspace is outside desktop access".into()));
            }
            sandbox.command(self.config.command.trim(), &args)?
        } else if let Some(sandbox) = &self.hermes_sandbox {
            if self.agent_name != "hermes" {
                return Err(AcpError::Initialize("Hermes sandbox requires the Hermes backend".into()));
            }
            let mut command = sandbox.command(self.config.command.trim(), &args)?;
            command.envs(&env);
            command
        } else if managed_vtcode {
            let executable = self.config.command.trim();
            crate::verify_reviewed_agent_executable("vtcode", executable)
                .map_err(AcpError::Initialize)?;
            let executable = Path::new(executable).canonicalize().map_err(AcpError::Spawn)?;
            let [Some(policy), Some(config), Some(home), Some(workspace)] = [
                "SHIJU_VTCODE_SANDBOX",
                "SHIJU_VTCODE_CONFIG",
                "SHIJU_VTCODE_HOME",
                "SHIJU_VTCODE_WORKSPACE",
            ].map(|key| env.remove(key))
            else {
                return Err(AcpError::Initialize("VT Code requires private launch files".into()));
            };
            if !cfg!(all(target_os = "macos", target_arch = "aarch64")) || args != ["acp"] {
                return Err(AcpError::Initialize("Unsupported managed VT Code launch".into()));
            }
            // Set private directories only on the child. The host and other ACP agents
            // retain their normal environment.
            env.insert("HOME".into(), home.clone());
            env.insert("TMPDIR".into(), home);
            env.insert("VT_ACP_ENABLED".into(), "1".into());
            env.insert("VT_ACP_ZED_ENABLED".into(), "1".into());
            let mut command = tokio::process::Command::new("/usr/bin/sandbox-exec");
            // VT Code initializes its tool registry from process cwd before session/new.
            command
                .current_dir(workspace)
                .args(["-f", &policy])
                .arg(executable)
                .args(["--config", &config]);
            command
        } else if self.managed_workspace_io() {
            let executable = self.config.command.trim();
            crate::verify_reviewed_agent_executable(&self.agent_name, executable)
                .map_err(AcpError::Initialize)?;
            let executable = Path::new(executable).canonicalize().map_err(AcpError::Spawn)?;
            let keys = if self.agent_name == "omp" {
                ["SHIJU_OMP_SANDBOX", "SHIJU_OMP_HOME", "SHIJU_OMP_WORKSPACE"]
            } else {
                ["SHIJU_MIMO_SANDBOX", "SHIJU_MIMO_HOME", "SHIJU_MIMO_WORKSPACE"]
            };
            let [Some(policy), Some(home), Some(workspace)] = keys
                    .map(|key| env.remove(key))
            else {
                return Err(AcpError::Initialize(format!("{} requires private launch files", self.agent_name)));
            };
            if !cfg!(all(target_os = "macos", target_arch = "aarch64")) || args != ["acp"] {
                return Err(AcpError::Initialize(format!("Unsupported managed {} launch", self.agent_name)));
            }
            env.insert("HOME".into(), home.clone());
            env.insert("TMPDIR".into(), home);
            if self.agent_name == "omp" && !self.mcp_servers.is_empty() {
                let launcher = Path::new(&policy).with_file_name("omp-launcher.py");
                let mut command = tokio::process::Command::new("/usr/bin/python3");
                command.current_dir(workspace).arg("-I").arg(launcher).arg(policy).arg(executable);
                command
            } else {
                let mut command = tokio::process::Command::new("/usr/bin/sandbox-exec");
                command.current_dir(workspace).args(["-f", &policy]).arg(executable);
                command
            }
        } else if let Some(entry) = glm_entry {
            let entry = entry?;
            if !args.is_empty() {
                return Err(AcpError::Initialize("Unsupported managed GLM launch arguments".into()));
            }
            let node_name = if cfg!(windows) { "node.exe" } else { "node" };
            let node = shell_command::resolve_executable(node_name)
                .filter(|path| path.is_absolute())
                .ok_or_else(|| AcpError::Initialize("Managed GLM requires Node.js".into()))?;
            let mut command = build_spawn_command(node, &env);
            command.arg(entry);
            command.env_remove("NODE_OPTIONS").env_remove("NODE_PATH");
            command
        } else {
            build_spawn_command(self.config.command.trim(), &env)
        };
        if self.agent_name == "copilot" {
            cmd.env_remove("COPILOT_ALLOW_ALL");
        }
        if self.droid_sandbox.is_none() && self.openclaw_sandbox.is_none() && (managed_claude || managed_auggie || managed_cursor || managed_droid || managed_private_provider) {
            // Do not inherit OAuth, cloud backends, Node injection or proxy credentials.
            // Keep the OS runtime paths. VT Code overrides its home in the child overlay.
            let runtime = [
                "PATH",
                "HOME",
                "USERPROFILE",
                "SystemRoot",
                "TMPDIR",
                "TMP",
                "TEMP",
                "LANG",
                "LC_ALL",
            ]
            .into_iter()
            .filter_map(|key| std::env::var_os(key).map(|value| (key, value)))
            .collect::<Vec<_>>();
            cmd.env_clear().envs(runtime);
            // Desktop processes may have only the system PATH. Restore the runtime
            // PATH used to resolve the adapter so its shebang can also find Node.
            #[cfg(not(windows))]
            shell_command::apply_augmented_path_tokio_command(&mut cmd);
            cmd.envs(&env);
        } else if self.agent_name == "sigit" {
            cmd.env_remove("VW_ACP_LAUNCH_REFERENCE")
                .env_remove("SIGIT_MANAGED_SKILL_DIRS");
        }
        if let Some(directory) = directory.filter(|_| self.droid_sandbox.is_none() && self.auggie_sandbox.is_none() && self.kilocode_sandbox.is_none() && self.hermes_sandbox.is_none() && self.opencode_sandbox.is_none() && self.openclaw_sandbox.is_none() && !managed_vtcode && !self.managed_workspace_io()) {
            cmd.current_dir(directory);
        }
        if self.agent_name == "opencode" && self.opencode_sandbox.is_none() {
            cmd.env_remove("VW_OC_RO");
        }
        cmd.args(&args);
        if self.managed_workspace_io() {
            match self.agent_name.as_str() {
                "mimo-code" => {
                    cmd.arg("--pure");
                }
                "omp" => {
                    let settings = env.get("SHIJU_OMP_SKILL_SETTINGS").ok_or_else(|| {
                        AcpError::Initialize("OMP requires private skill settings".into())
                    })?;
                    cmd.args(["--config", settings]);
                    cmd.args([
                        "--tools=read,write,edit",
                        "--no-extensions",
                        "--no-lsp",
                        "--offline",
                    ]);
                }
                _ => {}
            }
        }
        cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(if discard_stderr { Stdio::null() } else { Stdio::piped() })
            .kill_on_drop(true);
        #[cfg(unix)]
        unsafe {
            cmd.pre_exec(|| {
                if libc::setpgid(0, 0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        tracing::info!(
            target: "vw_acp",
            acp_agent = %self.agent_name,
            command = %self.config.command.trim(),
            args_count = args.len(),
            env_count = env.len(),
            "starting ACP agent process"
        );

        let mut child = cmd.spawn().map_err(AcpError::Spawn)?;
        let stderr_task = child.stderr.take().map(|mut stderr| {
            tokio::spawn(async move {
                let mut output = String::new();
                let _ = stderr.read_to_string(&mut output).await;
                output
            })
        });

        Ok(ProcessHandles { child, stderr_task })
    }

    /// Materialize only known official launch shapes; custom argv stays byte-for-byte intact.
    pub(super) fn launch_args(&self) -> Result<Vec<String>, AcpError> {
        let unrestricted = self.full_access && self.permission_mode == PermissionMode::ApproveAll;
        let restricted_pi = self.managed_restricted_pi();
        if restricted_pi {
            if !cfg!(all(target_os = "macos", target_arch = "aarch64"))
                || self.full_access
                || self.permission_mode != PermissionMode::ApproveReads
                || !self.config.args.is_empty()
                || self.desktop_workspace.is_none()
            {
                return Err(AcpError::Initialize("Invalid restricted Pi launch".into()));
            }
            crate::verify_reviewed_agent_executable("pi", self.config.command.trim())
                .map_err(AcpError::Initialize)?;
        }
        if self.auggie_sandbox.is_some()
            && (!cfg!(all(target_os = "macos", target_arch = "aarch64"))
                || self.agent_name != "auggie"
                || !self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
                || self.desktop_workspace.is_none()
                || self.full_access
                || self.permission_mode != PermissionMode::ApproveReads
                || self.config.args.first().is_none_or(|arg| arg != "--acp"))
        {
            return Err(AcpError::Initialize("Invalid restricted Auggie launch".into()));
        }
        if self.droid_sandbox.is_some()
            && (self.agent_name != "droid" || !self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
                || self.desktop_workspace.is_none()
                || self.full_access || self.permission_mode != PermissionMode::ApproveReads)
        {
            return Err(AcpError::Initialize("Invalid restricted Droid launch".into()));
        }
        if self.desktop_workspace.is_some()
            && self.droid_sandbox.is_none()            && self.auggie_sandbox.is_none()
            && !(self.agent_name == "hermes" && self.hermes_sandbox.is_some())
            && !(self.agent_name == "opencode" && self.opencode_sandbox.is_some())
            && !(self.agent_name == "kilocode" && self.kilocode_sandbox.is_some())
            && !(self.agent_name == "openclaw" && self.openclaw_sandbox.is_some())
            && !restricted_pi
        {
            Self::ensure_desktop_native_access(&self.agent_name, unrestricted)?;
        }
        if self.agent_name == "copilot" {
            crate::access_policy::copilot::validate_launch(&self.config.args, unrestricted)?;
        }
        if self.agent_name == "cursor" {
            crate::access_policy::cursor::validate_launch(&self.config.args, unrestricted)?;
        }
        if self.agent_name == "droid" {
            crate::access_policy::droid::validate_launch(&self.config.args, unrestricted)?;
        }
        if self.agent_name == "kiro"
            && self.config.args.iter().any(|arg| {
                arg == "--trust-all-tools"
                    || arg == "--trust-tools"
                    || arg.starts_with("--trust-all-tools=")
                    || arg.starts_with("--trust-tools=")
            })
        {
            return Err(AcpError::Initialize("Kiro startup tool trust cannot follow session permission changes; remove trust flags and use session policy presets".into()));
        }
        let command = std::path::Path::new(self.config.command.trim())
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let bypass = self.config.args.iter().any(|arg| {
            (arg == "-y"
                && (self.agent_name == "trae"
                    || matches!(command, "gemini" | "gemini-cli" | "qwen" | "qwen-code")))
                || matches!(
                    arg.as_str(),
                    "--yolo"
                        | "--skip-trust"
                        | "--dangerously-skip-permissions"
                        | "--dangerously-bypass-approvals-and-sandbox"
                )
                || matches!(
                    arg.as_str(),
                    "--approval-mode=yolo"
                        | "--permission-mode=bypass_permissions"
                        | "--permission-mode=bypassPermissions"
                )
        }) || self.config.args.windows(2).any(|pair| {
            matches!(
                (pair[0].as_str(), pair[1].as_str()),
                ("--approval-mode", "yolo")
                    | ("--permission-mode", "bypass_permissions" | "bypassPermissions")
            )
        });
        if self.agent_name == "gemini" {
            if bypass {
                return Err(AcpError::Initialize("Gemini native bypass and skip-trust lack safe session revocation; remove these arguments and use client approvals".into()));
            }
            // Do not turn a session grant into workspace trust or a native mode
            // that rewrites the model tools and system instruction.
            return Ok(self.config.args.clone());
        }
        if self.agent_name == "qwen" {
            if bypass {
                return Err(AcpError::Initialize("Qwen startup bypass cannot be safely revoked before session verification; remove bypass arguments".into()));
            }
            return Ok(self.config.args.clone());
        }
        if self.agent_name == "vibe"
            && (bypass
                || self
                    .config
                    .args
                    .iter()
                    .any(|arg| matches!(arg.as_str(), "--auto-approve" | "--agent=auto-approve"))
                || self
                    .config
                    .args
                    .windows(2)
                    .any(|pair| pair[0] == "--agent" && pair[1] == "auto-approve"))
        {
            return Err(AcpError::Initialize("Vibe startup auto-approval cannot follow session permission changes; remove bypass arguments and explicitly select the ACP agent with a current full access grant".into()));
        }
        if self.agent_name == "kimi" && bypass {
            return Err(AcpError::Initialize("Kimi startup bypass cannot follow session permission changes; remove bypass arguments and use session access controls".into()));
        }
        if self.agent_name == "qoder" && bypass {
            return Err(AcpError::Initialize("Qoder startup bypass cannot follow session permission changes; remove bypass arguments and use session access controls".into()));
        }
        if self.agent_name == "codebuddy" {
            let elevated_mode = self.config.args.iter().any(|arg| {
                arg == "-y"
                    || arg
                        .strip_prefix("--permission-mode=")
                        .is_some_and(|mode| !crate::access_policy::codebuddy::restricted_mode(mode))
            }) || self.config.args.windows(2).any(|pair| {
                pair[0] == "--permission-mode"
                    && !crate::access_policy::codebuddy::restricted_mode(&pair[1])
            });
            if bypass || elevated_mode {
                return Err(AcpError::Initialize("CodeBuddy startup elevated permissions cannot follow session access changes; remove bypass arguments and use session access controls".into()));
            }
        }
        if bypass && !unrestricted {
            return Err(AcpError::Initialize("launch arguments conflict with current full access authorization; remove explicit bypass arguments".into()));
        }
        Ok(self.config.args.clone())
    }

    /// 收尾 ACP 代理子进程并收集退出摘要。
    ///
    /// 函数先短暂等待自然退出，再依次发送终止和强制结束信号，最后读取 stderr
    /// 片段用于错误诊断。返回值不会包含敏感环境变量，只保留退出码、信号和
    /// stderr 文本供上层决定是否拼接到错误信息。
    #[cfg(test)]
    pub(crate) async fn finalize_child(
        &self,
        child: Child,
        stderr_task: Option<tokio::task::JoinHandle<String>>,
    ) -> FinalizedChild {
        let cleanup_targets = process_cleanup_targets(child.id());
        self.finalize_child_with_cleanup_targets(child, stderr_task, cleanup_targets).await
    }

    pub(super) async fn finalize_child_with_cleanup_targets(
        &self,
        mut child: Child,
        stderr_task: Option<tokio::task::JoinHandle<String>>,
        mut cleanup_targets: ProcessCleanupTargets,
    ) -> FinalizedChild {
        let process_id = child.id();
        let graceful_timeout = Duration::from_millis(500);
        let mut status = match timeout(graceful_timeout, child.wait()).await {
            Ok(Ok(status)) => Some(status),
            _ => {
                cleanup_targets.extend(process_cleanup_targets(process_id));
                send_terminate_signal_to_process_targets(&cleanup_targets);
                match timeout(Duration::from_secs(1), child.wait()).await {
                    Ok(Ok(status)) => Some(status),
                    Ok(Err(err)) => {
                        tracing::warn!(
                            target: "vw_acp",
                            acp_agent = %self.agent_name,
                            error = %err,
                            "failed to wait for ACP agent process exit"
                        );
                        None
                    }
                    Err(_) => {
                        cleanup_targets.extend(process_cleanup_targets(process_id));
                        send_kill_signal_to_process_targets(&cleanup_targets);
                        let _ = timeout(Duration::from_millis(500), child.wait()).await;
                        tracing::warn!(
                            target: "vw_acp",
                            acp_agent = %self.agent_name,
                            "timed out waiting for ACP agent process exit"
                        );
                        None
                    }
                }
            }
        };
        while status.is_none() {
            cleanup_targets.extend(process_cleanup_targets(process_id));
            send_kill_signal_to_process_targets(&cleanup_targets);
            match child.wait().await {
                Ok(exited) => status = Some(exited),
                Err(error) => {
                    tracing::error!(target: "vw_acp", acp_agent = %self.agent_name, ?process_id, %error,
                        "execution shutdown unconfirmed; retaining ACP runtime until exit can be confirmed");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
        cleanup_targets.extend(process_cleanup_targets(process_id));
        cleanup_process_targets(cleanup_targets).await;

        let stderr_output = match stderr_task {
            Some(task) => match timeout(Duration::from_millis(300), task).await {
                Ok(Ok(output)) => output,
                Ok(Err(_)) | Err(_) => String::new(),
            },
            None => String::new(),
        };

        let summary = child_exit_summary(status.as_ref());

        if stderr_output.trim().is_empty() {
            tracing::debug!(
                target: "vw_acp",
                acp_agent = %self.agent_name,
                exit_code = status.and_then(|value| value.code()).unwrap_or_default(),
                "ACP agent process exited"
            );
            return FinalizedChild { summary, stderr_output };
        }

        let stderr_preview: String = stderr_output.chars().take(400).collect();
        if self.verbose {
            tracing::warn!(
                target: "vw_acp",
                acp_agent = %self.agent_name,
                exit_code = status.and_then(|value| value.code()).unwrap_or_default(),
                stderr_len = stderr_output.len(),
                stderr = %stderr_output,
                "ACP agent process wrote to stderr"
            );
            return FinalizedChild { summary, stderr_output };
        }

        tracing::warn!(
            target: "vw_acp",
            acp_agent = %self.agent_name,
            exit_code = status.and_then(|value| value.code()).unwrap_or_default(),
            stderr_len = stderr_output.len(),
            stderr = %stderr_preview,
            "ACP agent process wrote to stderr"
        );
        FinalizedChild { summary, stderr_output }
    }
}

#[cfg(not(windows))]
fn insert_auth_env_if_missing(
    env: &mut std::collections::HashMap<String, String>,
    key: String,
    value: String,
) {
    env.entry(key).or_insert(value);
}

#[cfg(windows)]
fn insert_auth_env_if_missing(
    env: &mut std::collections::HashMap<String, String>,
    key: String,
    value: String,
) {
    if env.keys().any(|existing| existing.eq_ignore_ascii_case(&key)) {
        return;
    }

    env.insert(key, value);
}
