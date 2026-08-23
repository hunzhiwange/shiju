//! MCP startup state and status handling for the chat widget.
//!
//! The app server reports MCP server startup as per-server status updates. This
//! module keeps the TUI's buffered startup round state coherent and translates
//! those updates into status headers, warnings, and queued-input release points.
//! 本模块负责MCP 启动进度与状态展示。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。
//! 应用服务器按 MCP 服务发送启动更新；本模块把更新转换成状态标题、警告和待提交输入的释放时机。

use std::collections::BTreeSet;

use codex_app_server_protocol::McpServerStartupState;
use codex_app_server_protocol::McpServerStatusUpdatedNotification;

use super::ChatWidget;

const MCP_STARTUP_SINGLE_HEADER_PREFIX: &str = "Booting MCP server:";
const MCP_STARTUP_MULTI_HEADER_PREFIX: &str = "Starting MCP servers";

#[derive(Debug, Clone)]
pub(crate) enum McpStartupStatus {
    Starting,
    Ready,
    Failed { error: String },
    Cancelled,
}

impl ChatWidget {
    /// Record one MCP startup update, promoting it into either the active startup
    /// round or a buffered "next" round.
    ///
    /// This path has to deal with lossy app-server delivery. After
    /// `finish_mcp_startup()` or `finish_mcp_startup_after_lag()`, we briefly
    /// ignore incoming updates so stale events from the just-finished round do not
    /// reopen startup. While that guard is active we buffer updates for a possible
    /// next round, and only reactivate once the buffered set is coherent enough to
    /// treat as a fresh startup round.
    /// 记录一个MCP启动更新，将其提升到活动启动轮或缓冲的“下一轮”。此路径必须处理有损的应用-服务器交付。在`finish_mcp_startup ()`或`finish_mcp_startup_after_lag ()`之后，我们会短暂忽略传入的更新，因此刚刚完成的一轮过时事件不会重新打开启动。当该守卫处于活动状态时，我们会为下一轮可能的更新进行缓冲，并且仅重新激活一次
    /// 缓冲组足够连贯，可以作为新一轮启动。
    fn update_mcp_startup_status(
        &mut self,
        server: String,
        status: McpStartupStatus,
        complete_when_settled: bool,
    ) {
        let mut activated_pending_round = false;
        let startup_status = if self.mcp_startup_ignore_updates_until_next_start {
            // Ignore-mode buffers the next plausible round so stale post-finish
            // updates cannot immediately reopen startup. A fresh `Starting`
            // update resets the buffer only if we have not already seen a
            // pending-round `Starting`; this preserves valid interleavings like
            // `alpha: Starting -> alpha: Ready -> beta: Starting`.
            // Ignore-mode缓冲下一轮合理的轮次，因此过时的完成后更新无法立即重新打开启动。只有在我们还没有看到待定轮次的“Starting”时，新的“Starting”更新才会重置缓冲区；这保留了有效的交错，
            // 例如“alpha: Starting - > alpha: Ready - > beta: Starting”。
            if matches!(status, McpStartupStatus::Starting)
                && !self.mcp_startup_pending_next_round_saw_starting
            {
                self.mcp_startup_pending_next_round.clear();
                self.mcp_startup_allow_terminal_only_next_round = false;
            }
            self.mcp_startup_pending_next_round_saw_starting |=
                matches!(status, McpStartupStatus::Starting);
            self.mcp_startup_pending_next_round.insert(server, status);
            let Some(expected_servers) = &self.mcp_startup_expected_servers else {
                return;
            };
            let saw_full_round = expected_servers.is_empty()
                || expected_servers
                    .iter()
                    .all(|name| self.mcp_startup_pending_next_round.contains_key(name));
            let saw_starting = self
                .mcp_startup_pending_next_round
                .values()
                .any(|state| matches!(state, McpStartupStatus::Starting));
            if !(saw_full_round
                && (saw_starting || self.mcp_startup_allow_terminal_only_next_round))
            {
                return;
            }

            // The buffered map now looks like a complete next round, so promote it
            // to the active round and resume normal completion tracking.
            // 缓冲映射已构成完整的下一轮启动状态，将其提升为活动轮并恢复正常完成跟踪。
            self.mcp_startup_ignore_updates_until_next_start = false;
            self.mcp_startup_allow_terminal_only_next_round = false;
            self.mcp_startup_pending_next_round_saw_starting = false;
            activated_pending_round = true;
            std::mem::take(&mut self.mcp_startup_pending_next_round)
        } else {
            // Normal path: fold the update into the active round and surface
            // per-server failures immediately.
            // 正常路径：把更新并入活动轮次，并立即展示各服务器故障。
            let mut startup_status = self.mcp_startup_status.take().unwrap_or_default();
            if let McpStartupStatus::Failed { error } = &status {
                let already_reported = matches!(
                    startup_status.get(&server),
                    Some(McpStartupStatus::Failed { error: previous }) if previous == error
                );
                if !already_reported {
                    self.on_warning(error);
                }
            }
            startup_status.insert(server, status);
            startup_status
        };
        if activated_pending_round {
            // A promoted buffered round may already contain terminal failures.
            // 刚提升的缓冲轮次可能已经包含结束故障，需要立即展示。
            for state in startup_status.values() {
                if let McpStartupStatus::Failed { error } = state {
                    self.on_warning(error);
                }
            }
        }
        self.mcp_startup_status = Some(startup_status);
        self.update_task_running_state();

        // App-server-backed startup completes when every expected server has
        // reported a non-Starting status. Lag handling can force an earlier
        // settle via `finish_mcp_startup_after_lag()`.
        // 当每个预期的服务器都报告了非启动状态时，应用服务器支持的启动将完成。滞后处理可以通过“finish_mcp_startup_after_lag ()”强制提前结算。
        if complete_when_settled
            && let Some(current) = &self.mcp_startup_status
            && let Some(expected_servers) = &self.mcp_startup_expected_servers
            && !current.is_empty()
            && expected_servers
                .iter()
                .all(|name| current.contains_key(name))
            && current
                .values()
                .all(|state| !matches!(state, McpStartupStatus::Starting))
        {
            let mut failed = Vec::new();
            let mut cancelled = Vec::new();
            for (name, state) in current {
                match state {
                    McpStartupStatus::Ready => {}
                    McpStartupStatus::Failed { .. } => failed.push(name.clone()),
                    McpStartupStatus::Cancelled => cancelled.push(name.clone()),
                    McpStartupStatus::Starting => {}
                }
            }
            failed.sort();
            cancelled.sort();
            self.finish_mcp_startup(failed, cancelled);
            return;
        }
        if let Some(header) = self.mcp_startup_status_header() {
            self.set_status_header(header);
        }
        self.request_redraw();
    }

    pub(super) fn mcp_startup_status_header(&self) -> Option<String> {
        let current = self.mcp_startup_status.as_ref()?;
        let total = current.len();
        let mut starting: Vec<_> = current
            .iter()
            .filter_map(|(name, state)| {
                if matches!(state, McpStartupStatus::Starting) {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();
        starting.sort();
        let first = starting.first()?;
        let completed = total.saturating_sub(starting.len());
        let max_to_show = 3;
        let mut to_show: Vec<String> = starting
            .iter()
            .take(max_to_show)
            .map(ToString::to_string)
            .collect();
        if starting.len() > max_to_show {
            to_show.push("…".to_string());
        }
        Some(if total > 1 {
            format!(
                "{MCP_STARTUP_MULTI_HEADER_PREFIX} ({completed}/{total}): {}",
                to_show.join(", ")
            )
        } else {
            format!("{MCP_STARTUP_SINGLE_HEADER_PREFIX} {first}")
        })
    }

    pub(crate) fn set_mcp_startup_expected_servers<I>(&mut self, server_names: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.mcp_startup_expected_servers = Some(server_names.into_iter().collect());
    }

    pub(super) fn finish_mcp_startup(&mut self, failed: Vec<String>, cancelled: Vec<String>) {
        if !cancelled.is_empty() {
            self.on_warning(format!(
                "MCP startup interrupted. The following servers were not initialized: {}",
                cancelled.join(", ")
            ));
        }
        let mut parts = Vec::new();
        if !failed.is_empty() {
            parts.push(format!("failed: {}", failed.join(", ")));
        }
        if !parts.is_empty() {
            self.on_warning(format!("MCP startup incomplete ({})", parts.join("; ")));
        }

        let mcp_startup_owned_status = self.status_header_is_mcp_startup_owned();
        self.mcp_startup_status = None;
        self.mcp_startup_ignore_updates_until_next_start = true;
        self.mcp_startup_allow_terminal_only_next_round = false;
        self.mcp_startup_pending_next_round.clear();
        self.mcp_startup_pending_next_round_saw_starting = false;
        self.update_task_running_state();
        if self.input_queue.user_turn_pending_start {
            self.bottom_pane.set_task_running(/*running*/ true);
        }
        if self.bottom_pane.is_task_running() && mcp_startup_owned_status {
            self.restore_reasoning_status_header();
        }
        self.maybe_send_next_queued_input();
        self.request_redraw();
    }

    pub(crate) fn finish_mcp_startup_after_lag(&mut self) {
        if self.mcp_startup_ignore_updates_until_next_start {
            if self.mcp_startup_pending_next_round.is_empty() {
                self.mcp_startup_pending_next_round_saw_starting = false;
            }
            self.mcp_startup_allow_terminal_only_next_round = true;
        }

        let Some(current) = &self.mcp_startup_status else {
            return;
        };

        let mut failed = Vec::new();
        let mut cancelled = Vec::new();

        let mut server_names: BTreeSet<String> = current.keys().cloned().collect();
        if let Some(expected_servers) = &self.mcp_startup_expected_servers {
            server_names.extend(expected_servers.iter().cloned());
        }

        for name in server_names {
            match current.get(&name) {
                Some(McpStartupStatus::Failed { .. }) => failed.push(name),
                Some(McpStartupStatus::Cancelled) => cancelled.push(name),
                Some(McpStartupStatus::Ready | McpStartupStatus::Starting) | None => {}
            }
        }

        failed.sort();
        failed.dedup();
        cancelled.sort();
        cancelled.dedup();
        self.finish_mcp_startup(failed, cancelled);
        self.mcp_startup_allow_terminal_only_next_round = true;
    }

    pub(super) fn status_header_is_mcp_startup_owned(&self) -> bool {
        self.status_state
            .current_status
            .header
            .starts_with(MCP_STARTUP_SINGLE_HEADER_PREFIX)
            || self
                .status_state
                .current_status
                .header
                .starts_with(MCP_STARTUP_MULTI_HEADER_PREFIX)
    }

    /// Update startup state and retry installed-app discovery when its MCP server becomes ready.
    /// 更新启动状态，并在其MCP服务器就绪时重试安装应用程序发现。
    pub(super) fn on_mcp_server_status_updated(
        &mut self,
        notification: McpServerStatusUpdatedNotification,
    ) {
        let refresh_connector_mentions = notification.name == "codex_apps"
            && notification.status == McpServerStartupState::Ready;
        let status = match notification.status {
            McpServerStartupState::Starting => McpStartupStatus::Starting,
            McpServerStartupState::Ready => McpStartupStatus::Ready,
            McpServerStartupState::Failed => McpStartupStatus::Failed {
                error: notification.error.unwrap_or_else(|| {
                    format!("MCP client for `{}` failed to start", notification.name)
                }),
            },
            McpServerStartupState::Cancelled => McpStartupStatus::Cancelled,
        };
        self.update_mcp_startup_status(
            notification.name,
            status,
            /*complete_when_settled*/ true,
        );
        if refresh_connector_mentions {
            self.refresh_connector_mentions(/*force_refresh*/ false);
        }
    }
}
