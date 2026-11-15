//! 工作流运行订阅、事件合并与脱敏预览。

use super::*;

impl WorkflowState {
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
            title_source: None,
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
            title_source: Some("运行输出"),
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
        event: gateway_workflow::GatewayWorkflowStreamEvent,
    ) {
        match event {
            gateway_workflow::GatewayWorkflowStreamEvent::WorkflowStarted {
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
            gateway_workflow::GatewayWorkflowStreamEvent::NodeStarted {
                workflow_run_id,
                data,
                ..
            }
            | gateway_workflow::GatewayWorkflowStreamEvent::NodeFinished {
                workflow_run_id,
                data,
                ..
            } => {
                if let Some(preview) = self.saved_app_run_preview.as_mut() {
                    preview.run_id = non_empty_string(workflow_run_id).or(preview.run_id.take());
                    upsert_run_node(preview, data);
                }
            }
            gateway_workflow::GatewayWorkflowStreamEvent::TextChunk {
                workflow_run_id,
                text,
                ..
            } => {
                if let Some(preview) = self.saved_app_run_preview.as_mut() {
                    preview.run_id = non_empty_string(workflow_run_id).or(preview.run_id.take());
                    preview.text_output.push_str(&text);
                }
            }
            gateway_workflow::GatewayWorkflowStreamEvent::Message { answer, .. } => {
                if let Some(preview) = self.saved_app_run_preview.as_mut()
                    && preview.text_output.is_empty()
                    && !answer.is_empty()
                {
                    preview.text_output = answer;
                }
            }
            gateway_workflow::GatewayWorkflowStreamEvent::WorkflowFinished {
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
            gateway_workflow::GatewayWorkflowStreamEvent::MessageEnd { .. } => {
                self.finish_saved_app_run();
            }
            gateway_workflow::GatewayWorkflowStreamEvent::Error(error) => {
                self.fail_saved_app_run(WorkflowRunErrorPreview {
                    status: error.status,
                    code: error.code,
                    message: error.message,
                });
            }
            gateway_workflow::GatewayWorkflowStreamEvent::Unknown { .. } => {}
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
