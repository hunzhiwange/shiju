//! 活动运行器生命周期。

use super::{
    ActiveRunner,
    ActiveStream,
    ActiveStreamKind,
    Arc,
    AtomicBool,
    Editor,
    EditorAction,
    GatewayChatStreamEvent,
    RunnerControl,
    StreamMessage,
    V3Action,
    V3GatewayApp,
    WorkflowRunOwnership,
    mpsc,
    push_runner_error,
};

impl ActiveRunner {
    pub(crate) fn new() -> Self {
        Self::new_with_editor(Arc::new(|draft| {
            Editor::new().edit(draft).map_err(|error| error.to_string())
        }))
    }

    /// 用可注入的编辑器动作创建 runner，便于将外部进程与事件循环解耦。
    pub(crate) fn new_with_editor(editor: Arc<EditorAction>) -> Self {
        let (host_sender, host_receiver) = mpsc::channel();
        Self {
            active_stream: None,
            chat_submission: None,
            title_request_id: None,
            workflow_run: None,
            transcript_pager: None,
            native_copy_requested: false,
            session_bootstrap_in_flight: false,
            directory_switch_generation: 0,
            directory_switch_source: None,
            editor_in_flight: false,
            interaction_in_flight: false,
            file_search_generation: 0,
            file_search_query: None,
            session_diff_generation: 0,
            session_picker_generation: 0,
            session_picker_in_flight: None,
            doctor_generation: 0,
            cron_generation: 0,
            task_detail_generation: 0,
            workspace_files_generation: 0,
            hooks_generation: 0,
            memory_generation: 0,
            memory_mutation_generation: None,
            memory_snapshot_directory: None,
            memory_snapshot_query: None,
            hook_catalog_directory: None,
            hook_catalog: None,
            hook_mutation: None,
            editor,
            host_sender,
            host_receiver,
        }
    }

    /// 将界面操作转为宿主命令并执行，错误会投影为界面消息而非中止循环。
    pub(crate) fn execute_action(
        &mut self,
        app: &mut V3GatewayApp,
        action: V3Action,
    ) -> RunnerControl {
        let recovery = match &action {
            V3Action::Submit { text, attachments } | V3Action::Queue { text, attachments } => {
                Some((text.clone(), attachments.clone()))
            }
            _ => None,
        };
        if self.session_bootstrap_in_flight
            && matches!(action, V3Action::Submit { .. } | V3Action::Queue { .. })
        {
            if let Some((text, attachments)) = recovery {
                app.widget_mut().restore_submission(text.as_str(), attachments);
            }
            push_runner_error(app, "session history is still loading; retry in a moment");
            return RunnerControl::Continue;
        }
        if self.active_stream.is_some() && matches!(action, V3Action::Submit { .. }) {
            if let Some((text, attachments)) = recovery {
                app.widget_mut().restore_submission(text.as_str(), attachments);
            }
            push_runner_error(
                app,
                "a gateway stream is already active; queue the follow-up instead",
            );
            return RunnerControl::Continue;
        }
        if self.directory_switch_source.is_some()
            && matches!(action, V3Action::Submit { .. } | V3Action::Queue { .. })
        {
            if let Some((text, attachments)) = recovery {
                app.widget_mut().restore_submission(text.as_str(), attachments);
            }
            push_runner_error(app, "a directory switch is already in progress");
            return RunnerControl::Continue;
        }
        let command = match app.handle_action(action) {
            Ok(command) => command,
            Err(error) => {
                if let Some((text, attachments)) = recovery {
                    app.widget_mut().restore_submission(text.as_str(), attachments);
                }
                push_runner_error(app, error);
                app.show_next_interaction();
                return RunnerControl::Continue;
            }
        };
        self.execute_command(app, command, recovery)
    }

    #[cfg(test)]
    pub(crate) fn install_test_stream(
        &mut self,
        events: impl IntoIterator<Item = GatewayChatStreamEvent>,
    ) {
        let (sender, receiver) = mpsc::channel();
        for event in events {
            sender.send(StreamMessage::ChatEvent(event)).expect("test stream receiver");
        }
        self.active_stream = Some(ActiveStream {
            kind: ActiveStreamKind::Chat,
            request_id: "test-stream".to_string(),
            receiver,
            cancel: Arc::new(AtomicBool::new(false)),
        });
    }

    pub(super) fn start_next_queued_prompt(&mut self, app: &mut V3GatewayApp) {
        if self.active_stream.is_some() || self.native_copy_requested {
            return;
        }
        let Some((text, attachments)) = app.widget_mut().pop_queued_input() else {
            return;
        };
        let _ = self.execute_action(app, V3Action::Submit { text, attachments });
    }

    pub(super) fn refresh_pager(&mut self, app: &V3GatewayApp, _width: u16) {
        if let Some(pager) = self.transcript_pager.as_mut() {
            if pager.needs_source_refresh(app.widget().transcript_revision()) {
                pager.replace_committed(app.transcript_lines(u16::MAX, true));
            }
        }
    }

    pub(super) fn handle_pager_key(&mut self, key: crossterm::event::KeyEvent) {
        let Some(pager) = self.transcript_pager.as_mut() else {
            return;
        };
        pager.handle_key_event(key);
        if pager.is_done() {
            self.transcript_pager = None;
        }
    }

    #[cfg(test)]
    /// 返回当前是否有未结束的 Gateway 流。
    pub(crate) fn has_active_stream(&self) -> bool {
        self.active_stream.is_some()
    }

    #[cfg(test)]
    /// 返回活跃流的请求 ID，供停止与诊断路径关联。
    pub(crate) fn active_request_id(&self) -> Option<&str> {
        self.active_stream.as_ref().map(|stream| stream.request_id.as_str())
    }

    #[cfg(test)]
    pub(crate) fn active_stream_kind(&self) -> Option<&'static str> {
        self.active_stream.as_ref().map(|stream| match stream.kind {
            ActiveStreamKind::Chat => "chat",
            ActiveStreamKind::PlanRun => "plan-run",
            ActiveStreamKind::Workflow => "workflow",
        })
    }

    #[cfg(test)]
    pub(crate) fn last_workflow_run(&self) -> Option<&WorkflowRunOwnership> {
        self.workflow_run.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn last_workflow_request_id(&self) -> Option<&str> {
        self.workflow_run.as_ref().map(WorkflowRunOwnership::request_id)
    }

    /// 返回完整 transcript 阅读器是否已打开。
    pub(crate) fn has_transcript_pager(&self) -> bool {
        self.transcript_pager.is_some()
    }

    /// 返回外部编辑器动作是否仍在执行。
    pub(crate) fn editor_in_flight(&self) -> bool {
        self.editor_in_flight
    }

    /// 返回交互刷新或回复工作线程是否尚未结束。
    pub(crate) fn interaction_in_flight(&self) -> bool {
        self.interaction_in_flight
    }

    #[cfg(test)]
    pub(crate) fn directory_switch_in_flight(&self) -> bool {
        self.directory_switch_source.is_some()
    }

    #[cfg(test)]
    pub(crate) fn hook_catalog_contains(
        &self,
        plugin_id: &str,
        hook_hash: &str,
        trusted: bool,
    ) -> bool {
        self.hook_catalog
            .as_ref()
            .and_then(|catalog| catalog.find(plugin_id, hook_hash))
            .is_some_and(|hook| hook.trusted == trusted)
    }

    #[cfg(test)]
    pub(crate) fn hook_mutation_in_flight(&self) -> bool {
        self.hook_mutation.is_some()
    }

    #[cfg(test)]
    pub(crate) fn memory_mutation_in_flight(&self) -> bool {
        self.memory_mutation_generation.is_some()
    }
}
