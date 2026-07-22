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
        if self.config.command.trim().is_empty() {
            return Err(AcpError::EmptyCommand);
        }

        let mut env = self.config.env.clone();
        for (key, value) in build_agent_environment(&self.auth_credentials) {
            env.entry(key).or_insert(value);
        }

        let mut cmd = build_spawn_command(self.config.command.trim(), &env);
        cmd.args(self.config.args.iter())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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
            args_count = self.config.args.len(),
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
