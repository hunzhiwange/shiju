//! 宿主异步消息归并。

use super::{
    ActiveRunner,
    HOOK_CATALOG_SURFACE,
    HOST_MESSAGE_BUDGET,
    HostMessage,
    IntegrationKind,
    TryRecvError,
    V3DisplayEvent,
    V3GatewayApp,
    WORKSPACE_FILES_SURFACE,
    push_runner_error,
    spawn_interaction_refresh,
    spawn_session_bootstrap,
};

mod directory;
mod workflow;

impl ActiveRunner {
    /// 非阻塞汇集本地副作与 Gateway 流消息，并在流结束后尝试启动排队输入。
    pub(crate) fn drain(&mut self, app: &mut V3GatewayApp) -> bool {
        if app.take_interaction_refresh_requested() {
            self.request_interaction_refresh(app);
        }
        let host_work_remaining = self.drain_host_messages(app);
        let stream_work_remaining = self.drain_active_stream(app);
        if app.take_interaction_refresh_requested() {
            self.request_interaction_refresh(app);
        }
        host_work_remaining || stream_work_remaining
    }

    /// 在终端首帧可绘制后，后台加载显式指定的会话。
    pub(crate) fn request_session_bootstrap(&mut self, app: &mut V3GatewayApp) {
        if self.session_bootstrap_in_flight || app.runtime().session_id().is_none() {
            return;
        }
        self.session_bootstrap_in_flight = true;
        app.widget_mut().push_history(V3DisplayEvent::System {
            text: "正在加载指定会话…".to_string(),
        });
        spawn_session_bootstrap(app.runtime().clone(), self.host_sender.clone());
    }

    /// 在后台加载当前会话交互；并发事件会保留刷新标记供本轮结束后重试。
    pub(crate) fn request_interaction_refresh(&mut self, app: &mut V3GatewayApp) {
        if self.interaction_in_flight {
            app.request_interaction_refresh();
            return;
        }
        let Some(session_id) = app.state().session.session_id.clone() else {
            return;
        };
        self.interaction_in_flight = true;
        spawn_interaction_refresh(app.runtime().clone(), session_id, self.host_sender.clone());
    }

    fn drain_host_messages(&mut self, app: &mut V3GatewayApp) -> bool {
        for _ in 0..HOST_MESSAGE_BUDGET {
            match self.host_receiver.try_recv() {
                Ok(HostMessage::SessionTitleLoaded {
                    directory,
                    session_id,
                    request_id,
                    title,
                }) => {
                    if self.title_request_id.as_deref() == Some(request_id.as_str())
                        && app.runtime().directory() == directory
                        && app.runtime().session_id() == Some(session_id.as_str())
                        && app.state.session.title.trim().is_empty()
                    {
                        app.set_server_title(title);
                    }
                }
                Ok(HostMessage::SessionBootstrapFinished(result)) => {
                    self.session_bootstrap_in_flight = false;
                    match result {
                        Ok(state) => app.commit_initial_session_state(*state),
                        Err(error) => {
                            push_runner_error(
                                app,
                                format!("failed to load session history: {error}"),
                            );
                        }
                    }
                }
                Ok(HostMessage::DirectorySwitchFinished {
                    generation,
                    source_directory,
                    recovery_draft,
                    result,
                }) => {
                    self.finish_directory_switch(
                        app, generation, source_directory, recovery_draft, result,
                    );
                }
                Ok(HostMessage::EditorFinished(result)) => {
                    self.editor_in_flight = false;
                    match result {
                        Ok(Some(text)) => app.widget_mut().replace_draft(text.as_str()),
                        Ok(None) => {}
                        Err(error) => {
                            push_runner_error(app, format!("external editor failed: {error}"));
                        }
                    }
                }
                Ok(HostMessage::StopFinished { request_id, result }) => match result {
                    Ok(true) => {}
                    Ok(false) => push_runner_error(
                        app,
                        format!("gateway did not stop active stream {request_id}"),
                    ),
                    Err(error) => push_runner_error(
                        app,
                        format!("failed to stop active stream {request_id}: {error}"),
                    ),
                },
                Ok(HostMessage::PlanRunStopFinished { request_id, task_id, result }) => {
                    match result {
                        Ok(true) => {}
                        Ok(false) => push_runner_error(
                            app,
                            format!(
                                "gateway did not stop owned plan-run request {request_id} task {task_id}"
                            ),
                        ),
                        Err(error) => push_runner_error(
                            app,
                            format!(
                                "failed to stop owned plan-run request {request_id} task {task_id}: {error}"
                            ),
                        ),
                    }
                }
                Ok(HostMessage::InteractionsRefreshed(result)) => {
                    self.interaction_in_flight = false;
                    match result {
                        Ok(snapshot) => app.apply_interaction_snapshot(snapshot),
                        Err(error) => app.record_interaction_error(format!(
                            "failed to refresh pending interactions: {error}"
                        )),
                    }
                }
                Ok(HostMessage::PermissionReplyFinished { request_id, result }) => {
                    self.interaction_in_flight = false;
                    match result {
                        Ok(()) => app.complete_permission(request_id.as_str()),
                        Err(error) => {
                            app.record_interaction_error(format!(
                                "failed to reply to permission {request_id}: {error}"
                            ));
                            app.show_next_interaction();
                        }
                    }
                }
                Ok(HostMessage::QuestionReplyFinished { request_id, result }) => {
                    self.interaction_in_flight = false;
                    match result {
                        Ok(()) => app.complete_question(request_id.as_str()),
                        Err(error) => {
                            app.record_interaction_error(format!(
                                "failed to reply to question {request_id}: {error}"
                            ));
                            app.show_next_interaction();
                        }
                    }
                }
                Ok(HostMessage::WorkflowResumeFinished { pause_request_id, result }) => {
                    self.finish_workflow_resume(app, pause_request_id, result);
                }
                Ok(HostMessage::FileSearchFinished { generation, query, result }) => {
                    if generation != self.file_search_generation
                        || self.file_search_query.as_deref() != Some(query.as_str())
                    {
                        continue;
                    }
                    match result {
                        Ok(matches) => {
                            app.widget_mut().set_file_search_matches(query.as_str(), matches);
                        }
                        Err(error) => app.widget_mut().set_file_search_error(
                            query.as_str(),
                            format!("file search unavailable: {error}"),
                        ),
                    }
                }
                Ok(HostMessage::SessionDiffFinished { generation, session_id, result }) => {
                    if generation != self.session_diff_generation
                        || session_id.as_deref() != app.runtime().session_id()
                    {
                        continue;
                    }
                    match result {
                        Ok(diff) if diff.files.is_empty() => {
                            app.widget_mut().push_history(V3DisplayEvent::System {
                                text: "当前会话没有文件变化。".to_string(),
                            });
                        }
                        Ok(diff) => {
                            app.widget_mut().push_history(V3DisplayEvent::FileDiff { diff });
                        }
                        Err(error) => {
                            push_runner_error(app, format!("failed to load session diff: {error}"));
                        }
                    }
                }
                Ok(HostMessage::DoctorFinished { generation, directory, result }) => {
                    if generation != self.doctor_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    match result {
                        Ok(report) => {
                            app.widget_mut().push_history(V3DisplayEvent::System {
                                text: report.render_text(),
                            });
                        }
                        Err(error) => push_runner_error(
                            app,
                            format!("failed to load Gateway diagnostics: {error}"),
                        ),
                    }
                }
                Ok(HostMessage::CronDetailFinished { generation, directory, job_id, result }) => {
                    if generation != self.cron_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    match result {
                        Ok(detail) => app
                            .widget_mut()
                            .push_history(V3DisplayEvent::System { text: detail.render_text() }),
                        Err(error) => push_runner_error(
                            app,
                            format!("failed to load Cron detail {job_id}: {error}"),
                        ),
                    }
                }
                Ok(HostMessage::TaskDetailFinished { generation, directory, task_id, result }) => {
                    if generation != self.task_detail_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    match result {
                        Ok(detail) => app
                            .widget_mut()
                            .push_history(V3DisplayEvent::System { text: detail.render_text() }),
                        Err(error) => push_runner_error(
                            app,
                            format!("failed to load Scheduler task detail {task_id}: {error}"),
                        ),
                    }
                }
                Ok(HostMessage::WorkspaceFilesFinished {
                    generation,
                    directory,
                    requested_path,
                    result,
                }) => {
                    if generation != self.workspace_files_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    match result {
                        Ok(listing) if listing.path() == requested_path => {
                            app.widget_mut().show_selection_overlay(
                                WORKSPACE_FILES_SURFACE,
                                format!("浏览文件：{}", listing.path()),
                                listing.into_items(),
                            );
                        }
                        Ok(_) => push_runner_error(
                            app,
                            "workspace file listing did not match the current request",
                        ),
                        Err(error) => push_runner_error(
                            app,
                            format!("failed to browse workspace files: {error}"),
                        ),
                    }
                }
                Ok(HostMessage::WorkspaceFileReadFinished {
                    generation,
                    directory,
                    requested_path,
                    result,
                }) => {
                    if generation != self.workspace_files_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    match result {
                        Ok(text) if text.path() == requested_path => {
                            app.widget_mut()
                                .push_history(V3DisplayEvent::System { text: text.render_text() });
                        }
                        Ok(_) => push_runner_error(
                            app,
                            "workspace file response did not match the current request",
                        ),
                        Err(error) => push_runner_error(
                            app,
                            format!("failed to read workspace file: {error}"),
                        ),
                    }
                }
                Ok(HostMessage::HookCatalogFinished { generation, directory, result }) => {
                    if generation != self.hooks_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    match result {
                        Ok(catalog) => {
                            self.hook_catalog_directory = Some(directory);
                            self.hook_catalog = Some(catalog.clone());
                            app.widget_mut().show_selection_overlay(
                                HOOK_CATALOG_SURFACE,
                                "Hook 目录",
                                catalog.into_selection_items(),
                            );
                        }
                        Err(error) => {
                            push_runner_error(app, format!("failed to load Hook catalog: {error}"));
                        }
                    }
                }
                Ok(HostMessage::HookAuditFinished { generation, directory, result }) => {
                    if generation != self.hooks_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    match result {
                        Ok(audit) => app
                            .widget_mut()
                            .push_history(V3DisplayEvent::System { text: audit.render_text() }),
                        Err(error) => {
                            push_runner_error(app, format!("failed to load Hook audit: {error}"));
                        }
                    }
                }
                Ok(HostMessage::HookMutationFinished {
                    generation,
                    directory,
                    plugin_id,
                    hook_hash,
                    trusted,
                    result,
                }) => {
                    if generation != self.hooks_generation {
                        continue;
                    }
                    let Some(active) = self.hook_mutation.take() else {
                        continue;
                    };
                    if active.plugin_id != plugin_id
                        || active.hook_hash != hook_hash
                        || active.trusted != trusted
                    {
                        push_runner_error(
                            app,
                            "Hook mutation response did not match the active request",
                        );
                        continue;
                    }
                    if directory.as_path() != app.runtime().directory() {
                        continue;
                    }
                    match result {
                        Ok(refresh) => {
                            self.hook_catalog_directory = Some(directory);
                            self.hook_catalog = Some(refresh.catalog.clone());
                            app.widget_mut().push_history(V3DisplayEvent::System {
                                text: format!(
                                    "Hook {}: {plugin_id} {hook_hash}",
                                    if trusted { "已信任" } else { "已撤销信任" }
                                ),
                            });
                            app.widget_mut().push_history(V3DisplayEvent::System {
                                text: refresh.audit.render_text(),
                            });
                            app.widget_mut().show_selection_overlay(
                                HOOK_CATALOG_SURFACE,
                                "Hook 目录",
                                refresh.catalog.into_selection_items(),
                            );
                        }
                        Err(error) => push_runner_error(
                            app,
                            format!(
                                "failed to {} Hook {plugin_id} {hook_hash}: {error}",
                                if trusted { "trust" } else { "revoke" }
                            ),
                        ),
                    }
                }
                Ok(HostMessage::MemorySearchFinished { generation, directory, query, result }) => {
                    if generation != self.memory_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    match result {
                        Ok(items) => {
                            self.memory_snapshot_directory = Some(directory);
                            self.memory_snapshot_query.clone_from(&query);
                            app.replace_memory_items(items);
                            app.open_integrations(IntegrationKind::Memory);
                        }
                        Err(error) => push_runner_error(
                            app,
                            format!(
                                "failed to {} memory: {error}",
                                if query.is_some() { "query" } else { "refresh" }
                            ),
                        ),
                    }
                }
                Ok(HostMessage::MemoryMutationFinished {
                    generation,
                    directory,
                    query,
                    action,
                    key,
                    result,
                }) => {
                    if generation != self.memory_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    self.memory_mutation_generation = None;
                    match result {
                        Ok(items) => {
                            self.memory_snapshot_directory = Some(directory);
                            self.memory_snapshot_query.clone_from(&query);
                            app.replace_memory_items(items);
                            app.widget_mut().push_history(V3DisplayEvent::System {
                                text: format!(
                                    "记忆已{}: {key}",
                                    if action == "store" { "写入" } else { "删除" }
                                ),
                            });
                            app.open_integrations(IntegrationKind::Memory);
                        }
                        Err(error) => push_runner_error(
                            app,
                            format!("failed to {action} memory {key}: {error}"),
                        ),
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return false,
            }
        }
        true
    }
}
