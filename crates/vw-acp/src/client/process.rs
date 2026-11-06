//! ACP 代理子进程生命周期管理。
//!
//! 本模块封装代理进程的启动、进程组清理和 stderr 收集，让 actor 模块只关心
//! 运行时连接是否可用以及何时重启。

use super::*;

impl AcpClient {
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

        let args = self.launch_args()?;
        let mut env = self.config.env.clone();
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
        let managed_claude = managed_claude_settings(&self.config).is_some();
        if !managed_claude {
            for (key, value) in build_agent_environment(&self.auth_credentials) {
                insert_auth_env_if_missing(&mut env, key, value);
            }
        }

        let mut cmd = build_spawn_command(self.config.command.trim(), &env);
        if managed_claude {
            // Do not inherit OAuth, cloud backends, Node injection or proxy credentials.
            // Keep the OS runtime paths; HOME is preserved, never substituted.
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
            vw_shared::shell::apply_augmented_path_tokio_command(&mut cmd);
            cmd.envs(&env);
        }
        cmd.args(&args)
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
            (arg == "-y" && matches!(command, "gemini" | "gemini-cli" | "qwen" | "qwen-code"))
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
        if self.agent_name == "qoder" && bypass {
            return Err(AcpError::Initialize("Qoder startup bypass cannot follow session permission changes; remove bypass arguments and use session access controls".into()));
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
        let status = match timeout(graceful_timeout, child.wait()).await {
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
