//! # Workflow 核心状态操作
//!
//! 该模块提供核心状态读写、选择管理、脏状态判断、应用切换和浮层开关等通用逻辑。

use super::*;

impl WorkflowState {
    pub fn title(&self) -> &str {
        if self.active_app_id.is_none() {
            return "工作流";
        }

        if self.source_name.trim().is_empty() { "工作流" } else { self.source_name.as_str() }
    }

    pub fn active_app(&self) -> Option<&WorkflowAppEntry> {
        let active_id = self.active_app_id.as_deref()?;
        self.apps.iter().find(|app| app.id == active_id)
    }

    pub fn has_apps(&self) -> bool {
        !self.apps.is_empty()
    }

    pub fn begin_saved_apps_load(&mut self) {
        self.saved_apps_loading = true;
        self.saved_apps_error = None;
        self.saved_app_actions_menu_uuid = None;
    }

    pub fn finish_saved_apps_load(&mut self, result: Result<Vec<WorkflowSavedAppSummary>, String>) {
        self.saved_apps_loading = false;
        self.saved_apps_loaded = true;

        match result {
            Ok(apps) => {
                self.saved_apps = apps;
                self.saved_apps_error = None;
            }
            Err(error) => {
                self.saved_apps_error = Some(error);
            }
        }
    }

    pub fn begin_saved_app_open(&mut self, uuid: String) {
        self.opening_saved_app_uuid = Some(uuid);
        self.saved_apps_error = None;
        self.saved_app_actions_menu_uuid = None;
        self.close_floating_panels();
    }

    pub fn finish_saved_app_open(&mut self) {
        self.opening_saved_app_uuid = None;
    }

    pub fn begin_saved_app_delete(&mut self, uuid: String) {
        self.deleting_saved_app_uuid = Some(uuid);
        self.confirm_delete_saved_app_uuid = None;
        self.saved_app_actions_menu_uuid = None;
        self.saved_apps_error = None;
    }

    pub fn finish_saved_app_delete(&mut self) {
        self.deleting_saved_app_uuid = None;
    }

    pub fn open_saved_app_delete_confirm(&mut self, uuid: String) {
        self.confirm_delete_saved_app_uuid = Some(uuid);
        self.saved_app_actions_menu_uuid = None;
        self.saved_apps_error = None;
    }

    pub fn close_saved_app_delete_confirm(&mut self) {
        self.confirm_delete_saved_app_uuid = None;
    }

    pub fn remove_saved_app(&mut self, uuid: &str) -> bool {
        self.saved_apps.retain(|app| app.uuid != uuid);
        self.apps.retain(|app| app.local_uuid.as_deref() != Some(uuid));
        if self.confirm_delete_saved_app_uuid.as_deref() == Some(uuid) {
            self.confirm_delete_saved_app_uuid = None;
        }

        if self.saved_app_actions_menu_uuid.as_deref() == Some(uuid) {
            self.saved_app_actions_menu_uuid = None;
        }

        if self.copied_saved_app_uuid.as_deref() == Some(uuid) {
            self.copied_saved_app_uuid = None;
        }

        let removed_active = self.local_uuid.as_deref() == Some(uuid);
        if removed_active {
            self.active_app_id = None;
            self.active_is_dirty = false;
            self.source_name.clear();
            self.local_uuid = None;
            self.source_path = None;
            self.document = WorkflowDocument::default();
            self.environment_variables.clear();
            self.conversation_variables.clear();
            self.selected_node_id = None;
            self.selected_edge_id = None;
            self.connection_draft = None;
            self.undo_stack.clear();
            self.redo_stack.clear();
            self.saved_snapshot = None;
        }

        removed_active
    }

    pub fn show_saved_apps(&mut self) {
        self.persist_active_snapshot();
        self.active_app_id = None;
        self.close_floating_panels();
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.zoom_menu_open = false;
        self.app_editor = None;
        self.node_editor = None;
        self.variable_panel = None;
        self.variable_editor = None;
        self.error_message = None;
        self.saved_app_actions_menu_uuid = None;
    }

    pub fn set_saved_app_search_query(&mut self, query: String) {
        self.saved_app_search_query = query;
        self.saved_app_actions_menu_uuid = None;
    }

    pub fn toggle_saved_app_actions(&mut self, uuid: String) {
        if self.saved_app_actions_menu_uuid.as_deref() == Some(uuid.as_str()) {
            self.saved_app_actions_menu_uuid = None;
        } else {
            self.saved_app_actions_menu_uuid = Some(uuid);
        }
    }

    pub fn close_saved_app_actions(&mut self) {
        self.saved_app_actions_menu_uuid = None;
    }

    pub fn mark_saved_app_uuid_copied(&mut self, uuid: String) {
        self.copied_saved_app_uuid = Some(uuid);
        self.saved_app_actions_menu_uuid = None;
    }

    pub fn clear_saved_app_uuid_copied(&mut self, uuid: &str) {
        if self.copied_saved_app_uuid.as_deref() == Some(uuid) {
            self.copied_saved_app_uuid = None;
        }
    }

    pub fn open_saved_app_run_prompt(&mut self, uuid: String) {
        // 回填上次运行的 query，输入项的值在参数加载完成后再按名称覆盖。
        let query =
            self.last_run_drafts.get(&uuid).map(|draft| draft.query.clone()).unwrap_or_default();
        self.saved_app_run_prompt = Some(WorkflowRunPromptDraft {
            uuid,
            query,
            loading: true,
            inputs: Vec::new(),
            error: None,
        });
        self.saved_app_actions_menu_uuid = None;
        self.saved_apps_error = None;
    }

    pub fn close_saved_app_run_prompt(&mut self) {
        self.saved_app_run_prompt = None;
    }

    pub fn set_saved_app_run_query(&mut self, query: String) {
        if let Some(prompt) = self.saved_app_run_prompt.as_mut() {
            prompt.query = query;
            prompt.error = None;
        }
    }

    pub fn finish_saved_app_run_prompt_load(
        &mut self,
        uuid: &str,
        result: Result<Vec<WorkflowRunInputDraft>, String>,
    ) {
        let Some(prompt) = self.saved_app_run_prompt.as_mut() else {
            return;
        };
        if prompt.uuid != uuid {
            return;
        }

        prompt.loading = false;
        match result {
            Ok(mut inputs) => {
                // 结构以文档为准，但用上次运行缓存的同名输入值回填，方便重复测试。
                if let Some(cached) = self.last_run_drafts.get(uuid) {
                    for input in inputs.iter_mut() {
                        if let Some(remembered) = cached.inputs.iter().find(|cached_input| {
                            cached_input.name == input.name
                                && cached_input.input_type == input.input_type
                        }) {
                            input.value = remembered.value.clone();
                        }
                    }
                }
                prompt.inputs = inputs;
                prompt.error = None;
            }
            Err(error) => {
                prompt.inputs.clear();
                prompt.error = Some(error);
            }
        }
    }

    pub fn set_saved_app_run_input_value(&mut self, index: usize, value: String) {
        if let Some(prompt) = self.saved_app_run_prompt.as_mut() {
            if let Some(input) = prompt.inputs.get_mut(index) {
                input.value = value;
            }
            prompt.error = None;
        }
    }

    pub fn toggle_saved_app_run_input_value(&mut self, index: usize, value: bool) {
        if let Some(prompt) = self.saved_app_run_prompt.as_mut() {
            if let Some(input) = prompt.inputs.get_mut(index) {
                input.value = value.to_string();
            }
            prompt.error = None;
        }
    }

    pub fn set_saved_app_run_prompt_error(&mut self, error: String) {
        if let Some(prompt) = self.saved_app_run_prompt.as_mut() {
            prompt.error = Some(error);
        }
    }

    /// 缓存本次运行填写的参数草稿，供下次打开同一应用的运行弹窗时回填。
    pub fn remember_saved_app_run_draft(&mut self, prompt: WorkflowRunPromptDraft) {
        let mut draft = prompt;
        draft.loading = false;
        draft.error = None;
        self.last_run_drafts.insert(draft.uuid.clone(), draft);
    }

    pub fn begin_saved_app_run(
        &mut self,
        uuid: String,
        query: String,
        inputs: BTreeMap<String, serde_json::Value>,
    ) -> bool {
        if self.running_saved_app_uuid.is_some() {
            return false;
        }

        self.running_saved_app_uuid = Some(uuid.clone());
        self.running_saved_app_query = query;
        self.running_saved_app_inputs = inputs;
        self.saved_app_run_prompt = None;
        self.saved_app_actions_menu_uuid = None;
        self.saved_apps_error = None;
        self.saved_app_run_preview = Some(WorkflowRunPreview::running(uuid));
        self.run_error_copied = false;
        true
    }

    pub fn finish_saved_app_run(&mut self) {
        self.running_saved_app_uuid = None;
        if let Some(preview) = self.saved_app_run_preview.as_mut() {
            preview.is_finished = true;
            if preview.status == WorkflowRunStatus::Running {
                preview.status = WorkflowRunStatus::Succeeded;
            }
        }
    }

    pub fn fail_saved_app_run(&mut self, error: WorkflowRunErrorPreview) {
        self.running_saved_app_uuid = None;
        if let Some(preview) = self.saved_app_run_preview.as_mut() {
            preview.status = WorkflowRunStatus::Failed;
            preview.error = Some(error);
            preview.is_finished = true;
        }
    }

    pub fn cancel_saved_app_run(&mut self) {
        self.running_saved_app_uuid = None;
        if let Some(preview) = self.saved_app_run_preview.as_mut() {
            preview.status = WorkflowRunStatus::Canceled;
            preview.is_finished = true;
        }
    }

    pub fn retry_saved_app_run_preview(&mut self) -> bool {
        if self.running_saved_app_uuid.is_some() {
            return false;
        }

        let Some(preview) = self.saved_app_run_preview.as_ref() else {
            return false;
        };
        if !preview.is_finished
            || !matches!(preview.status, WorkflowRunStatus::Succeeded | WorkflowRunStatus::Failed)
        {
            return false;
        }

        let query = self.running_saved_app_query.trim().to_string();
        if query.is_empty() {
            return false;
        }

        self.begin_saved_app_run(preview.uuid.clone(), query, self.running_saved_app_inputs.clone())
    }

    pub fn close_saved_app_run_preview(&mut self) {
        self.running_saved_app_uuid = None;
        self.running_saved_app_query.clear();
        self.running_saved_app_inputs.clear();
        self.saved_app_run_preview = None;
        self.saved_app_run_detail = None;
        self.run_error_copied = false;
    }

    /// 打开运行预览中某个节点的“查看详细”弹窗，展示其完整原始数据。
    pub fn open_saved_app_run_node_detail(&mut self, index: usize) {
        let Some(node) =
            self.saved_app_run_preview.as_ref().and_then(|preview| preview.nodes.get(index))
        else {
            return;
        };
        let title = if node.title.trim().is_empty() { node.id.clone() } else { node.title.clone() };
        let content =
            serde_json::to_string_pretty(&node.data).unwrap_or_else(|_| node.data.to_string());
        self.saved_app_run_detail = Some(WorkflowRunDetailDialog {
            title,
            content: text_editor::Content::with_text(&content),
        });
    }

    /// 打开运行预览中文本输出的“查看详细”弹窗，展示完整文本。
    pub fn open_saved_app_run_output_detail(&mut self) {
        let Some(preview) = self.saved_app_run_preview.as_ref() else {
            return;
        };
        self.saved_app_run_detail = Some(WorkflowRunDetailDialog {
            title: "运行输出".to_string(),
            content: text_editor::Content::with_text(&preview.text_output),
        });
    }

    /// 在“查看详细”弹窗中执行只读编辑器动作，仅放行选中、复制与移动，拦截编辑。
    pub fn saved_app_run_detail_action(&mut self, action: text_editor::Action) {
        if action.is_edit() {
            return;
        }
        if let Some(detail) = self.saved_app_run_detail.as_mut() {
            detail.content.perform(action);
        }
    }

    pub fn close_saved_app_run_detail(&mut self) {
        self.saved_app_run_detail = None;
    }

    /// 标记运行错误详情已复制，并返回需要写入剪贴板的完整报告文本。
    /// 没有错误时返回 `None`，调用方据此跳过剪贴板与计时器任务。
    pub fn copy_saved_app_run_error(&mut self) -> Option<String> {
        let preview = self.saved_app_run_preview.as_ref()?;
        preview.error.as_ref()?;
        let report = preview.debug_text();
        self.run_error_copied = true;
        Some(report)
    }

    pub fn clear_saved_app_run_error_copied(&mut self) {
        self.run_error_copied = false;
    }

    pub fn saved_app_run_subscription_request(&self) -> Option<WorkflowRunSubscriptionRequest> {
        let uuid = self.running_saved_app_uuid.clone()?;
        Some(WorkflowRunSubscriptionRequest {
            uuid,
            query: self.running_saved_app_query.clone(),
            inputs: self.running_saved_app_inputs.clone(),
            user: "desktop-workflow".to_string(),
        })
    }

    pub fn apply_saved_app_run_event(
        &mut self,
        event: vw_gateway_client::GatewayWorkflowStreamEvent,
    ) {
        match event {
            vw_gateway_client::GatewayWorkflowStreamEvent::WorkflowStarted {
                workflow_run_id,
                data,
                ..
            } => {
                if let Some(preview) = self.saved_app_run_preview.as_mut() {
                    preview.run_id = non_empty_string(workflow_run_id).or(preview.run_id.take());
                    preview.status = WorkflowRunStatus::Running;
                    preview.is_finished = false;
                    merge_run_status(preview, &data);
                }
            }
            vw_gateway_client::GatewayWorkflowStreamEvent::NodeStarted {
                workflow_run_id,
                data,
                ..
            }
            | vw_gateway_client::GatewayWorkflowStreamEvent::NodeFinished {
                workflow_run_id,
                data,
                ..
            } => {
                if let Some(preview) = self.saved_app_run_preview.as_mut() {
                    preview.run_id = non_empty_string(workflow_run_id).or(preview.run_id.take());
                    upsert_run_node(preview, data);
                }
            }
            vw_gateway_client::GatewayWorkflowStreamEvent::TextChunk {
                workflow_run_id,
                text,
                ..
            } => {
                if let Some(preview) = self.saved_app_run_preview.as_mut() {
                    preview.run_id = non_empty_string(workflow_run_id).or(preview.run_id.take());
                    preview.text_output.push_str(&text);
                }
            }
            vw_gateway_client::GatewayWorkflowStreamEvent::Message { answer, .. } => {
                if let Some(preview) = self.saved_app_run_preview.as_mut()
                    && preview.text_output.is_empty()
                    && !answer.is_empty()
                {
                    preview.text_output = answer;
                }
            }
            vw_gateway_client::GatewayWorkflowStreamEvent::WorkflowFinished {
                workflow_run_id,
                data,
                ..
            } => {
                if let Some(preview) = self.saved_app_run_preview.as_mut() {
                    preview.run_id = non_empty_string(workflow_run_id).or(preview.run_id.take());
                    merge_run_status(preview, &data);
                }
                self.finish_saved_app_run();
            }
            vw_gateway_client::GatewayWorkflowStreamEvent::MessageEnd { .. } => {
                self.finish_saved_app_run();
            }
            vw_gateway_client::GatewayWorkflowStreamEvent::Error(error) => {
                self.fail_saved_app_run(WorkflowRunErrorPreview {
                    status: error.status,
                    code: error.code,
                    message: error.message,
                });
            }
            vw_gateway_client::GatewayWorkflowStreamEvent::Unknown { .. } => {}
        }
    }

    pub fn has_start_node(&self) -> bool {
        self.document.nodes.iter().any(|node| node.block_type == "start")
    }

    pub fn active_meta(&self) -> Option<&WorkflowAppMeta> {
        self.active_app().map(|app| &app.meta)
    }

    pub fn selected_node(&self) -> Option<&WorkflowNode> {
        self.selected_node_id.as_deref().and_then(|id| self.document.node(id))
    }

    pub fn selected_edge(&self) -> Option<&WorkflowEdge> {
        self.selected_edge_id.as_deref().and_then(|id| self.document.edge(id))
    }

    pub fn environment_variable(&self, id: &str) -> Option<&WorkflowEnvironmentVariable> {
        self.environment_variables.iter().find(|variable| variable.id == id)
    }

    pub fn conversation_variable(&self, id: &str) -> Option<&WorkflowConversationVariable> {
        self.conversation_variables.iter().find(|variable| variable.id == id)
    }

    pub fn set_error(&mut self, error: impl Into<String>) {
        self.error_message = Some(error.into());
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    pub fn select_node(&mut self, id: String) {
        self.selected_node_id = Some(id);
        self.selected_edge_id = None;
        self.context_menu = None;
        self.sync_selection_flags();
    }

    pub fn select_edge(&mut self, id: String) {
        self.selected_edge_id = Some(id);
        self.selected_node_id = None;
        self.context_menu = None;
        self.sync_selection_flags();
    }

    pub fn clear_selection(&mut self) {
        self.selected_node_id = None;
        self.selected_edge_id = None;
        self.context_menu = None;
        self.sync_selection_flags();
    }

    pub fn current_history_snapshot(&self) -> Option<WorkflowHistorySnapshot> {
        self.active_app_id.as_ref()?;

        Some(WorkflowHistorySnapshot {
            meta: WorkflowAppMeta {
                name: self.source_name.clone(),
                ..self.active_meta().cloned().unwrap_or_default()
            },
            document: self.document.clone(),
            environment_variables: self.environment_variables.clone(),
            conversation_variables: self.conversation_variables.clone(),
            pan: self.pan,
            zoom: self.zoom,
            selected_node_id: self.selected_node_id.clone(),
            selected_edge_id: self.selected_edge_id.clone(),
        })
    }

    pub(super) fn push_history_snapshot(&mut self, snapshot: WorkflowHistorySnapshot) {
        if self.undo_stack.last() != Some(&snapshot) {
            self.undo_stack.push(snapshot);
            if self.undo_stack.len() > WORKFLOW_HISTORY_LIMIT {
                self.undo_stack.remove(0);
            }
        }
        self.redo_stack.clear();
    }

    pub(super) fn push_undo_snapshot(&mut self) {
        let Some(snapshot) = self.current_history_snapshot() else {
            return;
        };

        self.push_history_snapshot(snapshot);
    }

    pub(super) fn refresh_dirty_state(&mut self) {
        let Some(current) = self.current_history_snapshot() else {
            self.active_is_dirty = false;
            return;
        };

        self.active_is_dirty = self.saved_snapshot.as_ref() != Some(&current);
    }

    pub(super) fn restore_history_snapshot(&mut self, snapshot: WorkflowHistorySnapshot) {
        self.source_name = snapshot.meta.name.clone();
        self.document = snapshot.document;
        self.environment_variables = snapshot.environment_variables;
        self.conversation_variables = snapshot.conversation_variables;
        self.pan = snapshot.pan;
        self.zoom = snapshot.zoom;
        self.selected_node_id = snapshot.selected_node_id;
        self.selected_edge_id = snapshot.selected_edge_id;
        self.connection_draft = None;
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.dragging_node_id = None;
        self.drag_pending_snapshot = None;
        self.app_editor = None;
        self.node_editor = None;
        self.variable_panel = None;
        self.variable_editor = None;

        if let Some(active_id) = self.active_app_id.clone()
            && let Some(app) = self.apps.iter_mut().find(|app| app.id == active_id)
        {
            app.meta = WorkflowAppMeta { name: self.source_name.clone(), ..snapshot.meta };
        }

        self.sync_selection_flags();
        self.refresh_dirty_state();
    }

    pub fn apply_loaded(&mut self, loaded: LoadedWorkflow, window_size: (f32, f32)) {
        self.persist_active_snapshot();
        let app_id = generate_app_id();
        let selected_node_id =
            loaded.document.nodes.iter().find(|node| node.selected).map(|node| node.id.clone());
        let selected_edge_id =
            loaded.document.edges.iter().find(|edge| edge.selected).map(|edge| edge.id.clone());

        let (pan, zoom) = if loaded.had_viewport {
            (
                Vector::new(loaded.document.viewport.x, loaded.document.viewport.y),
                loaded.document.viewport.zoom.clamp(0.1, 4.0),
            )
        } else {
            fitted_viewport(&loaded.document, window_size)
        };
        let saved_snapshot = WorkflowHistorySnapshot {
            meta: loaded.app_meta.clone(),
            document: loaded.document.clone(),
            environment_variables: loaded.environment_variables.clone(),
            conversation_variables: loaded.conversation_variables.clone(),
            pan,
            zoom,
            selected_node_id: selected_node_id.clone(),
            selected_edge_id: selected_edge_id.clone(),
        };

        self.apps.push(WorkflowAppEntry {
            id: app_id.clone(),
            local_uuid: loaded.local_uuid.clone(),
            meta: loaded.app_meta.clone(),
            source_path: loaded.source_path.clone(),
            raw_root: loaded.raw_root.clone(),
            document: loaded.document.clone(),
            environment_variables: loaded.environment_variables.clone(),
            conversation_variables: loaded.conversation_variables.clone(),
            pan,
            zoom,
            selected_node_id: selected_node_id.clone(),
            selected_edge_id: selected_edge_id.clone(),
            connection_draft: None,
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            saved_snapshot: saved_snapshot.clone(),
        });

        self.active_app_id = Some(app_id);
        self.active_is_dirty = false;

        self.source_name = loaded.source_name.clone();
        self.local_uuid = loaded.local_uuid.clone();
        self.source_path = loaded.source_path.clone();
        self.document = loaded.document;
        self.environment_variables = loaded.environment_variables;
        self.conversation_variables = loaded.conversation_variables;
        self.selected_node_id = selected_node_id;
        self.selected_edge_id = selected_edge_id;
        self.connection_draft = None;
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.variable_panel = None;
        self.variable_editor = None;
        self.error_message = None;

        self.pan = pan;
        self.zoom = zoom;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.saved_snapshot = Some(saved_snapshot);
        self.dragging_node_id = None;
        self.drag_pending_snapshot = None;

        self.status_message = Some(match self.source_path.as_deref() {
            Some(path) => format!("已加载 {}", path),
            None => format!("已加载 {}", self.title()),
        });
        self.sync_selection_flags();
    }

    pub fn replace_active_loaded(&mut self, loaded: LoadedWorkflow, window_size: (f32, f32)) {
        let Some(active_id) = self.active_app_id.clone() else {
            self.apply_loaded(loaded, window_size);
            return;
        };

        let selected_node_id =
            loaded.document.nodes.iter().find(|node| node.selected).map(|node| node.id.clone());
        let selected_edge_id =
            loaded.document.edges.iter().find(|edge| edge.selected).map(|edge| edge.id.clone());
        let (pan, zoom) = if loaded.had_viewport {
            (
                Vector::new(loaded.document.viewport.x, loaded.document.viewport.y),
                loaded.document.viewport.zoom.clamp(0.1, 4.0),
            )
        } else {
            fitted_viewport(&loaded.document, window_size)
        };
        let saved_snapshot = WorkflowHistorySnapshot {
            meta: loaded.app_meta.clone(),
            document: loaded.document.clone(),
            environment_variables: loaded.environment_variables.clone(),
            conversation_variables: loaded.conversation_variables.clone(),
            pan,
            zoom,
            selected_node_id: selected_node_id.clone(),
            selected_edge_id: selected_edge_id.clone(),
        };

        if let Some(app) = self.apps.iter_mut().find(|app| app.id == active_id) {
            app.meta = loaded.app_meta.clone();
            app.local_uuid = loaded.local_uuid.clone();
            app.source_path = loaded.source_path.clone();
            app.raw_root = loaded.raw_root.clone();
            app.document = loaded.document.clone();
            app.environment_variables = loaded.environment_variables.clone();
            app.conversation_variables = loaded.conversation_variables.clone();
            app.pan = pan;
            app.zoom = zoom;
            app.selected_node_id = selected_node_id.clone();
            app.selected_edge_id = selected_edge_id.clone();
            app.connection_draft = None;
            app.is_dirty = false;
            app.undo_stack.clear();
            app.redo_stack.clear();
            app.saved_snapshot = saved_snapshot.clone();
        }

        self.active_is_dirty = false;
        self.source_name = loaded.source_name.clone();
        self.local_uuid = loaded.local_uuid.clone();
        self.source_path = loaded.source_path.clone();
        self.document = loaded.document;
        self.environment_variables = loaded.environment_variables;
        self.conversation_variables = loaded.conversation_variables;
        self.pan = pan;
        self.zoom = zoom;
        self.selected_node_id = selected_node_id;
        self.selected_edge_id = selected_edge_id;
        self.connection_draft = None;
        self.app_editor = None;
        self.node_editor = None;
        self.app_editor = None;
        self.node_editor = None;
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.variable_panel = None;
        self.variable_editor = None;
        self.error_message = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.saved_snapshot = Some(saved_snapshot);
        self.dragging_node_id = None;
        self.drag_pending_snapshot = None;
        self.status_message = Some(format!("已重新载入 {}", self.title()));
        self.sync_selection_flags();
    }

    pub fn select_app(&mut self, id: &str) -> bool {
        if self.active_app_id.as_deref() == Some(id) {
            return false;
        }

        self.persist_active_snapshot();
        let Some(app) = self.apps.iter().find(|app| app.id == id).cloned() else {
            return false;
        };

        self.active_app_id = Some(app.id.clone());
        self.load_entry_into_current(&app);
        self.status_message = Some(format!("已切换到 {}", self.title()));
        true
    }

    pub fn select_app_by_local_uuid(&mut self, uuid: &str) -> bool {
        let Some(app_id) = self
            .apps
            .iter()
            .find(|app| app.local_uuid.as_deref() == Some(uuid))
            .map(|app| app.id.clone())
        else {
            return false;
        };

        self.select_app(&app_id)
    }

    pub fn open_create_editor(&mut self) {
        self.node_editor = None;
        self.variable_editor = None;
        self.variable_panel = None;
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.app_editor = Some(WorkflowAppEditorDraft {
            mode: WorkflowAppEditorMode::Create,
            name: self.next_app_name(),
            description: String::new(),
            icon: "🤖".to_string(),
            use_icon_as_answer_icon: false,
            max_active_requests_input: "0".to_string(),
        });
    }

    pub fn open_edit_editor(&mut self, id: Option<&str>) {
        let target_id = id.map(|value| value.to_string()).or_else(|| self.active_app_id.clone());
        let Some(target_id) = target_id else {
            return;
        };

        if let Some(app) = self.apps.iter().find(|app| app.id == target_id) {
            self.node_editor = None;
            self.variable_editor = None;
            self.variable_panel = None;
            self.context_menu = None;
            self.quick_insert_panel_open = false;
            self.action_menu_open = false;
            self.zoom_menu_open = false;
            self.app_editor = Some(WorkflowAppEditorDraft {
                mode: WorkflowAppEditorMode::Edit(target_id),
                name: app.meta.name.clone(),
                description: app.meta.description.clone(),
                icon: app.meta.icon.clone(),
                use_icon_as_answer_icon: app.meta.use_icon_as_answer_icon,
                max_active_requests_input: app.meta.max_active_requests.to_string(),
            });
        }
    }

    pub fn close_editor(&mut self) {
        self.app_editor = None;
    }

    pub fn toggle_action_menu(&mut self) {
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.zoom_menu_open = false;
        self.action_menu_open = !self.action_menu_open;
    }

    pub fn toggle_zoom_menu(&mut self) {
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.action_menu_open = false;
        self.zoom_menu_open = !self.zoom_menu_open;
    }

    pub fn close_floating_panels(&mut self) {
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.quick_insert_panel_open = false;
    }

    pub fn toggle_quick_insert_panel(&mut self) {
        self.context_menu = None;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.quick_insert_panel_open = !self.quick_insert_panel_open;
    }

    pub fn close_quick_insert_panel(&mut self) {
        self.quick_insert_panel_open = false;
    }

    pub fn update_active_source_path(&mut self, path: String) {
        self.source_path = Some(path.clone());

        if let Some(active_id) = self.active_app_id.clone()
            && let Some(app) = self.apps.iter_mut().find(|app| app.id == active_id)
        {
            app.source_path = Some(path);
        }

        if let Some(snapshot) = self.current_history_snapshot() {
            self.saved_snapshot = Some(snapshot.clone());
            self.active_is_dirty = false;

            if let Some(active_id) = self.active_app_id.clone()
                && let Some(app) = self.apps.iter_mut().find(|app| app.id == active_id)
            {
                app.saved_snapshot = snapshot;
                app.is_dirty = false;
            }
        }
    }

    pub fn update_active_local_uuid(&mut self, uuid: String) {
        self.local_uuid = Some(uuid.clone());

        if let Some(active_id) = self.active_app_id.clone()
            && let Some(app) = self.apps.iter_mut().find(|app| app.id == active_id)
        {
            app.local_uuid = Some(uuid);
        }

        if let Some(snapshot) = self.current_history_snapshot() {
            self.saved_snapshot = Some(snapshot.clone());
            self.active_is_dirty = false;

            if let Some(active_id) = self.active_app_id.clone()
                && let Some(app) = self.apps.iter_mut().find(|app| app.id == active_id)
            {
                app.saved_snapshot = snapshot;
                app.is_dirty = false;
            }
        }
    }

    pub(super) fn sync_selection_flags(&mut self) {
        let selected_node_id = self.selected_node_id.as_deref();
        for node in &mut self.document.nodes {
            node.selected = Some(node.id.as_str()) == selected_node_id;
        }

        let selected_edge_id = self.selected_edge_id.as_deref();
        for edge in &mut self.document.edges {
            edge.selected = Some(edge.id.as_str()) == selected_edge_id;
        }
    }

    fn load_entry_into_current(&mut self, app: &WorkflowAppEntry) {
        self.source_name = app.meta.name.clone();
        self.local_uuid = app.local_uuid.clone();
        self.source_path = app.source_path.clone();
        self.document = app.document.clone();
        self.environment_variables = app.environment_variables.clone();
        self.conversation_variables = app.conversation_variables.clone();
        self.pan = app.pan;
        self.zoom = app.zoom;
        self.selected_node_id = app.selected_node_id.clone();
        self.selected_edge_id = app.selected_edge_id.clone();
        self.connection_draft = app.connection_draft.clone();
        self.undo_stack = app.undo_stack.clone();
        self.redo_stack = app.redo_stack.clone();
        self.saved_snapshot = Some(app.saved_snapshot.clone());
        self.active_is_dirty = app.is_dirty;
        self.app_editor = None;
        self.node_editor = None;
        self.variable_panel = None;
        self.variable_editor = None;
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.dragging_node_id = None;
        self.drag_pending_snapshot = None;
        self.error_message = None;
        self.sync_selection_flags();
    }

    fn next_app_name(&self) -> String {
        let base = "未命名应用";
        if !self.apps.iter().any(|app| app.meta.name == base) {
            return base.to_string();
        }

        let mut index = 2;
        loop {
            let candidate = format!("{} {}", base, index);
            if !self.apps.iter().any(|app| app.meta.name == candidate) {
                return candidate;
            }
            index += 1;
        }
    }
}

impl WorkflowRunPreview {
    fn running(uuid: String) -> Self {
        Self {
            uuid,
            run_id: None,
            status: WorkflowRunStatus::Running,
            nodes: Vec::new(),
            text_output: String::new(),
            error: None,
            is_finished: false,
        }
    }

    pub fn debug_text(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("source: {}", self.error_source()));
        lines.push(format!("uuid: {}", self.uuid));
        lines.push(format!("run_id: {}", self.run_id.as_deref().unwrap_or("pending")));
        lines.push(format!("status: {}", workflow_run_error_status(self.error.as_ref())));
        lines.push(format!("finished: {}", self.is_finished));

        if let Some(node) = self.failed_node() {
            lines.push(format!("failed_node: {} ({})", node.title, node.id));
        }

        if let Some(error) = &self.error {
            lines.push(format!("code: {}", error.code));
            lines.push(format!("message: {}", error.message));
        }

        if !self.nodes.is_empty() {
            lines.push("nodes:".to_string());
            for node in &self.nodes {
                let status = node.status.as_deref().unwrap_or("pending");
                lines.push(format!("- {} ({}) status: {}", node.title, node.id, status));
                lines.push(format!("  data: {}", node.debug_data_text()));
            }
        }

        lines.join("\n")
    }

    fn failed_node(&self) -> Option<&WorkflowRunNodePreview> {
        self.nodes
            .iter()
            .find(|node| matches!(node.status.as_deref(), Some("failed") | Some("error")))
    }

    fn error_source(&self) -> &'static str {
        if let Some(error) = &self.error {
            if error.code == "transport_error" {
                return "connection";
            }
            if error.status > 0 {
                return "gateway";
            }
        }

        if self.failed_node().is_some() { "node" } else { "workflow" }
    }
}

impl WorkflowRunNodePreview {
    pub fn debug_data_text(&self) -> String {
        sanitized_debug_value(&self.data).to_string()
    }
}

fn non_empty_string(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

fn json_string_field(data: &serde_json::Value, field: &str) -> Option<String> {
    data.get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
}

fn upsert_run_node(preview: &mut WorkflowRunPreview, data: serde_json::Value) {
    let Some(id) = json_string_field(&data, "node_id").or_else(|| json_string_field(&data, "id"))
    else {
        return;
    };
    let title = json_string_field(&data, "title").unwrap_or_else(|| id.clone());
    let status = json_string_field(&data, "status");

    let node_failed = matches!(status.as_deref(), Some("failed") | Some("error"));
    let node_failed_message = node_failed.then(|| format!("节点失败: {title}"));

    if let Some(node) = preview.nodes.iter_mut().find(|node| node.id == id) {
        node.title = title;
        node.status = status;
        node.data = data;
    } else {
        preview.nodes.push(WorkflowRunNodePreview { id, title, status, data });
    }

    if node_failed {
        preview.status = WorkflowRunStatus::Failed;
        if preview.error.is_none() {
            preview.error = Some(WorkflowRunErrorPreview {
                status: 0,
                code: "node_failed".to_string(),
                message: node_failed_message.unwrap_or_else(|| "节点失败".to_string()),
            });
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn workflow_run_inputs_from_document(
    document: &WorkflowDocument,
) -> Vec<WorkflowRunInputDraft> {
    document
        .nodes
        .iter()
        .filter(|node| node.block_type == "start")
        .flat_map(workflow_run_inputs_from_start_node)
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn workflow_run_inputs_from_start_node(node: &WorkflowNode) -> Vec<WorkflowRunInputDraft> {
    let Some(variables) = node
        .raw_node
        .as_mapping()
        .and_then(|node| node.get(&Value::String("data".to_string())))
        .and_then(Value::as_mapping)
        .and_then(|data| data.get(&Value::String("variables".to_string())))
        .and_then(Value::as_sequence)
    else {
        return Vec::new();
    };

    variables
        .iter()
        .map(super::start_variable_drafts::build_start_variable_draft)
        .filter(|variable| !variable.variable.trim().is_empty())
        .map(workflow_run_input_from_start_variable)
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn workflow_run_input_from_start_variable(
    variable: WorkflowStartVariableDraft,
) -> WorkflowRunInputDraft {
    let value = if matches!(variable.input_type.as_str(), "file" | "file-list") {
        variable.default_file_values.join("\n")
    } else {
        variable.default_value.clone()
    };

    WorkflowRunInputDraft {
        name: variable.variable.trim().to_string(),
        label: if variable.label.trim().is_empty() {
            variable.variable.trim().to_string()
        } else {
            variable.label.trim().to_string()
        },
        input_type: variable.input_type,
        required: variable.required,
        hidden: variable.hidden,
        options: variable.options,
        value,
        placeholder: variable.placeholder,
        hint: variable.hint,
        max_length_input: variable.max_length_input,
    }
}

fn merge_run_status(preview: &mut WorkflowRunPreview, data: &serde_json::Value) {
    match json_string_field(data, "status").as_deref() {
        Some("failed") | Some("error") => preview.status = WorkflowRunStatus::Failed,
        Some("canceled") | Some("cancelled") => preview.status = WorkflowRunStatus::Canceled,
        Some("succeeded") | Some("success") | Some("finished") => {
            preview.status = WorkflowRunStatus::Succeeded;
        }
        _ => {}
    }
}

fn workflow_run_error_status(error: Option<&WorkflowRunErrorPreview>) -> String {
    let Some(error) = error else {
        return "n/a".to_string();
    };

    if error.status == 0 && error.code == "transport_error" {
        "transport".to_string()
    } else if error.status == 0 {
        "n/a".to_string()
    } else {
        error.status.to_string()
    }
}

fn sanitized_debug_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(sanitized_debug_value).collect())
        }
        serde_json::Value::Object(map) => {
            let filtered = map
                .iter()
                .filter(|(key, _)| !is_sensitive_debug_key(key))
                .map(|(key, value)| (key.clone(), sanitized_debug_value(value)))
                .collect();
            serde_json::Value::Object(filtered)
        }
        _ => value.clone(),
    }
}

fn is_sensitive_debug_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.starts_with("__vw_")
        || lower.contains("token")
        || lower.contains("secret")
        || lower.contains("password")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower == "authorization"
        || lower == "cookie"
}
