//! Agent turn lifecycle and runtime bookkeeping for `ChatWidget`.
//!
//! This module owns task start/completion state, runtime metrics, plan updates,
//! and final-message separator handling.
//! 本模块负责代理轮次状态、指标、计划与结束分隔。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。
//! 底部面板的“任务运行中”标志由代理轮次、审查模式和 MCP 启动状态共同派生。

use super::*;

const LEGACY_SAFETY_ACCESS_BLOCK_PREFIX: &str =
    "Invalid prompt: we've limited access to this content for safety reasons.";
const BIO_POLICY_SAFETY_ACCESS_BLOCK_PREFIX: &str =
    "This content was flagged for possible biological risk.";

fn is_safety_access_block_message(message: &str) -> bool {
    message.starts_with(LEGACY_SAFETY_ACCESS_BLOCK_PREFIX)
        || message.starts_with(BIO_POLICY_SAFETY_ACCESS_BLOCK_PREFIX)
}

impl ChatWidget {
    fn clear_guardian_review_status(&mut self) {
        self.status_state.pending_guardian_review_status.clear();
        if self.status_state.current_status.is_guardian_review() {
            let header = self
                .mcp_startup_status_header()
                .unwrap_or_else(|| String::from("Working"));
            self.set_status_header(header);
        }
    }

    /// Synchronize the bottom-pane "task running" indicator with the current lifecycles.
    ///
    /// The bottom pane only has one running flag, but this module treats it as a derived state of
    /// both the agent turn lifecycle and MCP startup lifecycle.
    /// 将底部面板"任务运行"指标与当前生命周期同步. 底部面板只有一个运行的标志，但这个模块将它作为代理轮次生命周期和MCP启动生命周期的衍生状态.
    pub(super) fn update_task_running_state(&mut self) {
        self.bottom_pane.set_task_running(
            self.turn_lifecycle.agent_turn_running
                || self.review.is_review_mode
                || self.mcp_startup_status.is_some(),
        );
        self.refresh_plan_mode_nudge();
        self.refresh_status_surfaces();
    }

    pub(super) fn collect_runtime_metrics_delta(&mut self) {
        if let Some(delta) = self.session_telemetry.runtime_metrics_summary() {
            self.apply_runtime_metrics_delta(delta);
        }
    }

    pub(super) fn apply_runtime_metrics_delta(&mut self, delta: RuntimeMetricsSummary) {
        let should_log_timing = has_websocket_timing_metrics(delta);
        self.turn_runtime_metrics.merge(delta);
        if should_log_timing {
            self.log_websocket_timing_totals(delta);
        }
    }

    pub(super) fn log_websocket_timing_totals(&mut self, delta: RuntimeMetricsSummary) {
        if let Some(label) = history_cell::runtime_metrics_label(delta.responses_api_summary()) {
            self.add_plain_history_lines(vec![
                vec!["• ".dim(), format!("WebSocket timing: {label}").dark_gray()].into(),
            ]);
        }
    }

    pub(super) fn refresh_runtime_metrics(&mut self) {
        self.collect_runtime_metrics_delta();
    }

    // Raw reasoning uses the same flow as summarized reasoning
    // 原始推理使用与摘要推理相同的流程

    pub(super) fn on_task_started(&mut self) {
        self.input_queue.user_turn_pending_start = false;
        self.reset_safety_buffering_for_turn_start();
        self.turn_lifecycle.start(Instant::now());
        self.transcript.reset_turn_flags();
        self.adaptive_chunking.reset();
        if self.plan_stream_controller.take().is_some() {
            self.request_pending_usage_output_insertion_after_stream_shutdown();
        }
        self.turn_runtime_metrics = RuntimeMetricsSummary::default();
        self.session_telemetry.reset_runtime_metrics();
        self.bottom_pane.clear_quit_shortcut_hint();
        self.quit_shortcut_expires_at = None;
        self.quit_shortcut_key = None;
        self.update_task_running_state();
        self.status_state.retry_status_header = None;
        self.clear_active_hook_cell();
        self.status_state.pending_status_indicator_restore = false;
        self.bottom_pane
            .set_interrupt_hint_visible(/*visible*/ true);
        self.status_state.terminal_title_status_kind = TerminalTitleStatusKind::Working;
        if self.mcp_startup_status.is_none() || !self.status_header_is_mcp_startup_owned() {
            self.set_status_header(String::from("Working"));
        }
        self.reasoning_summary_parts.clear();
        self.reasoning_buffer.clear();
        self.reasoning_header = None;
        self.set_ambient_pet_notification(
            crate::pets::PetNotificationKind::Running,
            /*body*/ None,
        );
        self.request_redraw();
    }

    pub(super) fn on_task_complete(
        &mut self,
        last_agent_message: Option<String>,
        duration_ms: Option<i64>,
        from_replay: bool,
    ) {
        self.input_queue.submit_pending_steers_after_interrupt = false;
        let sanitized_last_agent_message = last_agent_message.as_deref().map(|message| {
            parse_assistant_markdown(message, self.config.cwd.as_path()).visible_markdown
        });
        // For desktop notifications: prefer the notification payload, fall back to
        // the item-level copy source if present, otherwise send an empty string.
        // 对于桌面通知: 偏爱通知有效载荷， 如果存在则返回到项目级副本源， 否则发送空字符串。
        let notification_response = sanitized_last_agent_message
            .as_ref()
            .filter(|message| !message.is_empty())
            .cloned()
            .or_else(|| {
                if self.transcript.saw_copy_source_this_turn {
                    self.transcript.last_agent_markdown.clone()
                } else {
                    None
                }
            })
            .unwrap_or_default();
        self.transcript.saw_copy_source_this_turn = false;
        // If a stream is currently active, finalize it.
        // 若仍有活动流式单元，先将其完整结束。
        self.flush_answer_stream_with_separator();
        if let Some(mut controller) = self.plan_stream_controller.take() {
            let had_live_tail = controller.has_live_tail();
            self.clear_active_stream_tail();
            let (cell, source) = controller.finalize();
            if !had_live_tail && let Some(cell) = cell {
                self.add_boxed_history(cell);
            }
            if let Some(source) = source {
                self.note_stream_consolidation_queued();
                self.app_event_tx
                    .send(AppEvent::ConsolidateProposedPlan(source));
            }
            self.request_pending_usage_output_insertion_after_stream_shutdown();
        }
        self.flush_unified_exec_wait_streak();
        self.flush_completed_command_activity();
        if !from_replay {
            self.collect_runtime_metrics_delta();
            let runtime_metrics =
                (!self.turn_runtime_metrics.is_empty()).then_some(self.turn_runtime_metrics);
            let show_work_separator = self.transcript.had_work_activity
                && (self.transcript.needs_final_message_separator || runtime_metrics.is_some());
            if show_work_separator || runtime_metrics.is_some() {
                let elapsed_seconds = if show_work_separator {
                    duration_ms
                        .and_then(|duration_ms| u64::try_from(duration_ms).ok())
                        .map(|duration_ms| duration_ms / 1_000)
                        .or_else(|| {
                            self.bottom_pane
                                .status_widget()
                                .map(crate::status_indicator_widget::StatusIndicatorWidget::elapsed_seconds)
                        })
                } else {
                    None
                };
                self.add_to_history(history_cell::FinalMessageSeparator::new(
                    elapsed_seconds,
                    runtime_metrics,
                ));
            }
            self.turn_runtime_metrics = RuntimeMetricsSummary::default();
            self.transcript.needs_final_message_separator = false;
            self.transcript.had_work_activity = false;
            self.request_status_line_branch_refresh();
            self.request_status_line_git_summary_refresh();
            self.refresh_thread_usage_after_turn();
        }
        // Mark task stopped and request redraw now that all content is in history.
        // 标记任务已停止， 现在所有内容都已载入历史。
        self.status_state.pending_status_indicator_restore = false;
        self.input_queue.user_turn_pending_start = false;
        self.clear_active_hook_cell();
        self.clear_guardian_review_status();
        self.turn_lifecycle.finish();
        self.clear_safety_buffering();
        self.update_task_running_state();
        self.running_commands.clear();
        self.suppressed_exec_calls.clear();
        self.last_unified_wait = None;
        self.unified_exec_wait_streak = None;
        if !from_replay {
            let body = Notification::agent_turn_preview(&notification_response);
            self.set_ambient_pet_notification(crate::pets::PetNotificationKind::Review, body);
        }
        self.request_redraw();

        let had_pending_steers = !self.input_queue.pending_steers.is_empty();
        self.refresh_pending_input_preview();

        if !from_replay && !self.has_queued_follow_up_messages() && !had_pending_steers {
            self.maybe_prompt_plan_implementation();
        }
        // Keep this flag for replayed completion events so a subsequent live TurnComplete can
        // still show the prompt once after thread switch replay.
        // 回放完成事件时保留此标志，使任务切换回放后的实时 `TurnComplete` 仍能且仅能显示一次提示。
        if !from_replay {
            self.transcript.saw_plan_item_this_turn = false;
        }
        // If there is a queued user message, send exactly one now to begin the next turn.
        // 若有排队的用户消息，只发送一条来启动下一轮次。
        let follow_up_started = self.maybe_send_next_queued_input();
        let active_goal_continuing = self
            .current_goal_status
            .as_ref()
            .is_some_and(GoalStatusState::is_active);
        // Emit a notification when the agent is truly waiting for the user.
        // Queued follow-up input and active goal continuation both start the
        // next turn immediately, so notifying at that boundary would feel like
        // a false "needs attention".
        // 代理在真正等待用户时发送通知. 排队后续输入和积极目标续作都立即开始下一个轮次，因此在那个边界通知会感觉像一个虚假的"需要注意".
        if !follow_up_started && !active_goal_continuing {
            self.notify(Notification::AgentTurnComplete {
                response: notification_response,
            });
        }

        self.maybe_show_pending_rate_limit_prompt();
    }

    pub(super) fn maybe_prompt_plan_implementation(&mut self) {
        if !self.collaboration_modes_enabled() {
            return;
        }
        if self.has_queued_follow_up_messages() {
            return;
        }
        if self.active_mode_kind() != ModeKind::Plan {
            return;
        }
        if !self.transcript.saw_plan_item_this_turn {
            return;
        }
        if !self.bottom_pane.no_modal_or_popup_active() {
            return;
        }

        if matches!(
            self.rate_limit_switch_prompt,
            RateLimitSwitchPromptState::Pending
        ) {
            return;
        }

        self.open_plan_implementation_prompt();
    }

    pub(super) fn open_plan_implementation_prompt(&mut self) {
        let default_mask = collaboration_modes::default_mode_mask(self.model_catalog.as_ref());
        let context_usage_label = self.plan_implementation_context_usage_label();

        self.bottom_pane
            .show_selection_view(plan_implementation::selection_view_params(
                default_mask,
                self.transcript.latest_proposed_plan_markdown.as_deref(),
                context_usage_label.as_deref(),
            ));
        self.notify(Notification::PlanModePrompt {
            title: PLAN_IMPLEMENTATION_TITLE.to_string(),
        });
    }

    /// Returns a context-used label for the plan implementation prompt.
    ///
    /// The footer reports context remaining because it is ambient status, but
    /// this prompt is asking whether to discard prior conversation state before
    /// implementing a plan. Reporting used context makes the cleanup tradeoff
    /// explicit. A fully fresh or unknown context window returns no label so
    /// the clear-context option does not imply urgency without evidence.
    /// 返回计划实施提示所用的“已使用上下文”标签。页脚展示剩余上下文作为环境状态，而此提示在询问实施计划前是否丢弃先前对话，因此展示已用量能明确清理取舍。全新或未知的上下文窗口不返回标签，避免在没有依据时暗示清理紧迫性。
    pub(super) fn plan_implementation_context_usage_label(&self) -> Option<String> {
        let info = self.token_info.as_ref()?;
        let percent = self.context_remaining_percent(info);

        let used_tokens = self.context_used_tokens(info, percent.is_some());
        if let Some(percent) = percent {
            let used_percent = 100 - percent.clamp(0, 100);
            if used_percent <= 0 {
                return None;
            }
            return Some(format!("{used_percent}% used"));
        }

        if let Some(tokens) = used_tokens
            && tokens > 0
        {
            return Some(format!("{} used", format_tokens_compact(tokens)));
        }

        None
    }

    pub(crate) fn has_queued_follow_up_messages(&self) -> bool {
        self.input_queue.has_queued_follow_up_messages()
    }

    pub(super) fn handle_app_server_steer_rejected_error(
        &mut self,
        codex_error_info: &AppServerCodexErrorInfo,
    ) -> bool {
        matches!(
            codex_error_info,
            AppServerCodexErrorInfo::ActiveTurnNotSteerable { .. }
        ) && self.enqueue_rejected_steer()
    }

    /// Finalize any active exec as failed and stop/clear agent-turn UI state.
    ///
    /// This does not clear MCP startup tracking, because MCP startup can overlap with turn cleanup
    /// and should continue to drive the bottom-pane running indicator while it is in progress.
    /// 将所有活动执行单元结束为失败，并停止、清理代理轮次界面状态。这里不清除 MCP 启动跟踪，因为 MCP 启动可能与轮次清理重叠，进行中仍须驱动底部面板的运行指示器。
    pub(super) fn finalize_turn(&mut self) {
        self.clear_safety_buffering();
        // Drop preview-only stream tail content on any termination path before
        // failed-cell finalization, so transient tail cells are never persisted.
        // 任一终止路径都先丢弃仅供预览的流式尾部，再完成失败单元，避免把临时尾部单元写入历史。
        self.clear_active_stream_tail();
        // Ensure any spinner is replaced by a red ✗ and flushed into history.
        // 确保任何旋转器被红色的QQ所取代，并冲入历史.
        self.finalize_active_cell_as_failed();
        // Turn-scoped hook rows are transient live state; once the turn is over,
        // do not leave an orphaned running row behind if no matching completion
        // event arrived before cancellation.
        // 转镜钩行是短暂的活状态;一旦轮次结束，如果取消前没有到达匹配的完成事件，则不要留下一个孤行运行.
        self.clear_active_hook_cell();
        // Reset running state and clear streaming buffers.
        // 重置运行状态和清流缓冲.
        self.input_queue.user_turn_pending_start = false;
        self.clear_guardian_review_status();
        self.turn_lifecycle.finish();
        self.update_task_running_state();
        self.running_commands.clear();
        self.suppressed_exec_calls.clear();
        self.last_unified_wait = None;
        self.unified_exec_wait_streak = None;
        self.adaptive_chunking.reset();
        self.stream_controller = None;
        self.plan_stream_controller = None;
        self.request_pending_usage_output_insertion_after_stream_shutdown();
        self.status_state.pending_status_indicator_restore = false;
        self.safety_buffering_prompt = None;
        self.request_status_line_branch_refresh();
        self.request_status_line_git_summary_refresh();
        self.refresh_thread_usage_after_turn();
        self.maybe_show_pending_rate_limit_prompt();
    }

    pub(super) fn on_server_overloaded_error(&mut self, message: String) {
        self.input_queue.submit_pending_steers_after_interrupt = false;
        self.finalize_turn();

        let message = if message.trim().is_empty() {
            "Codex is currently experiencing high load.".to_string()
        } else {
            message
        };

        self.add_to_history(history_cell::new_warning_event(message));
        self.request_redraw();
        self.maybe_send_next_queued_input();
    }

    fn on_error(&mut self, message: String) {
        self.input_queue.submit_pending_steers_after_interrupt = false;
        self.flush_answer_stream_with_separator();
        self.finalize_turn();
        self.add_to_history(history_cell::new_error_event(message));
        self.set_ambient_pet_notification(
            crate::pets::PetNotificationKind::Failed,
            /*body*/ None,
        );
        self.request_redraw();

        // After an error ends the turn, try sending the next queued input.
        // 错误结束轮次后， 请尝试发送下一个队列输入。
        self.maybe_send_next_queued_input();
    }

    pub(crate) fn handle_turn_start_rejection(&mut self, message: String) -> bool {
        if !self.input_queue.user_turn_pending_start {
            return false;
        }
        self.on_error(message);
        true
    }

    pub(super) fn on_cyber_policy_error(&mut self) {
        self.input_queue.submit_pending_steers_after_interrupt = false;
        self.finalize_turn();
        self.add_to_history(history_cell::new_cyber_policy_error_event());
        self.request_redraw();

        // After an error ends the turn, try sending the next queued input.
        // 错误结束轮次后， 请尝试发送下一个队列输入。
        self.maybe_send_next_queued_input();
    }

    pub(super) fn on_rate_limit_error(&mut self, error_kind: RateLimitErrorKind, message: String) {
        let usage_limit_error = matches!(error_kind, RateLimitErrorKind::UsageLimit);
        let rate_limit_reached_type = self.codex_rate_limit_reached_type.map(|kind| {
            if usage_limit_error {
                match kind {
                    RateLimitReachedType::WorkspaceOwnerCreditsDepleted => {
                        RateLimitReachedType::WorkspaceOwnerUsageLimitReached
                    }
                    RateLimitReachedType::WorkspaceMemberCreditsDepleted => {
                        RateLimitReachedType::WorkspaceMemberUsageLimitReached
                    }
                    other => other,
                }
            } else {
                kind
            }
        });
        self.codex_rate_limit_reached_type = rate_limit_reached_type;
        match rate_limit_reached_type {
            Some(RateLimitReachedType::WorkspaceOwnerCreditsDepleted) => {
                self.on_error(
                    "You're out of credits. Your workspace is out of credits. Add credits to continue using Codex."
                        .to_string(),
                );
            }
            Some(RateLimitReachedType::WorkspaceOwnerUsageLimitReached) => {
                self.on_error(
                    "Usage limit reached. You've reached your usage limit. Increase your limits to continue using codex."
                        .to_string(),
                );
            }
            Some(RateLimitReachedType::WorkspaceMemberCreditsDepleted) => {
                self.on_error(message);
                self.open_workspace_owner_nudge_prompt(AddCreditsNudgeCreditType::Credits);
            }
            Some(RateLimitReachedType::WorkspaceMemberUsageLimitReached) => {
                self.on_error(message);
                self.open_workspace_owner_nudge_prompt(AddCreditsNudgeCreditType::UsageLimit);
            }
            Some(RateLimitReachedType::RateLimitReached) | None => {
                self.on_error(message);
            }
        }
    }

    pub(super) fn handle_non_retry_error(
        &mut self,
        message: String,
        codex_error_info: Option<AppServerCodexErrorInfo>,
    ) {
        if codex_error_info == Some(AppServerCodexErrorInfo::MisalignmentPolicyViolation) {
            self.on_misalignment_policy_violation();
        } else if codex_error_info
            .as_ref()
            .is_some_and(|info| self.handle_app_server_steer_rejected_error(info))
        {
        } else if codex_error_info
            .as_ref()
            .is_some_and(is_app_server_cyber_policy_error)
        {
            self.on_cyber_policy_error();
        } else if is_safety_access_block_message(&message)
            || serde_json::from_str::<serde_json::Value>(&message).is_ok_and(|response| {
                response["error"]["code"].as_str() == Some("bio_policy")
                    || response["error"]["message"]
                        .as_str()
                        .is_some_and(is_safety_access_block_message)
            })
        {
            self.input_queue.submit_pending_steers_after_interrupt = false;
            self.finalize_turn();
            self.add_to_history(history_cell::new_safety_access_block_event());
            self.request_redraw();
            self.maybe_send_next_queued_input();
        } else if let Some(info) = codex_error_info
            .as_ref()
            .and_then(app_server_rate_limit_error_kind)
        {
            match info {
                RateLimitErrorKind::ServerOverloaded => self.on_server_overloaded_error(message),
                RateLimitErrorKind::UsageLimit | RateLimitErrorKind::Generic => {
                    self.on_rate_limit_error(info, message)
                }
            }
        } else {
            self.on_error(message);
        }
    }

    pub(super) fn on_warning(&mut self, message: impl Into<String>) {
        let message = message.into();
        if !self.warning_display_state.should_display(&message) {
            return;
        }
        self.add_to_history(history_cell::new_warning_event(message));
        self.request_redraw();
    }

    pub(super) fn on_app_server_model_verification(
        &mut self,
        verifications: &[AppServerModelVerification],
    ) {
        if verifications.contains(&AppServerModelVerification::TrustedAccessForCyber) {
            self.on_warning(TRUSTED_ACCESS_FOR_CYBER_VERIFICATION_WARNING);
        }
    }

    pub(super) fn on_plan_update(&mut self, update: UpdatePlanArgs) {
        self.transcript.saw_plan_update_this_turn = true;
        let total = update.plan.len();
        let completed = update
            .plan
            .iter()
            .filter(|item| match &item.status {
                StepStatus::Completed => true,
                StepStatus::Pending | StepStatus::InProgress => false,
            })
            .count();
        self.transcript.last_plan_progress = (total > 0).then_some((completed, total));
        self.refresh_status_surfaces();
        self.add_to_history(history_cell::new_plan_update(update));
    }

    pub(super) fn interrupted_turn_message(&self, reason: TurnAbortReason) -> String {
        if reason == TurnAbortReason::BudgetLimited {
            return "Goal budget reached - the turn was stopped.".to_string();
        }

        "Conversation interrupted - tell the model what to do differently. Something went wrong? Hit `/feedback` to report the issue.".to_string()
    }
}
