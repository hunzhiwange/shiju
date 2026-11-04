//! Only typed counters and the immutable template digest cross the UI boundary.
use super::*;
use vw_api_types::workflow::{VideoEditProgress, WorkflowNodeRunStatus};
#[derive(Default)]
pub(super) struct Progress {
    value: VideoEditProgress,
    chunks: BTreeMap<String, u8>,
    terminal: bool,
    last: Option<VideoEditProgress>,
}
impl Progress {
    pub(super) fn new(source: &str) -> Self {
        use sha2::{Digest, Sha256};
        Self {
            value: VideoEditProgress {
                template_sha256: hex::encode(Sha256::digest(source.as_bytes())),
                ..Default::default()
            },
            ..Default::default()
        }
    }
    pub(super) fn ready(&self) -> bool {
        !self.value.run_id.is_empty()
    }
    pub(super) fn finish(&mut self, succeeded: bool, cancelled: bool) {
        self.terminal = true;
        self.value.stage = if cancelled {
            "已取消"
        } else if succeeded {
            "预览完成"
        } else {
            "失败"
        }
        .into();
    }

    pub(super) fn observe(&mut self, event: &WorkflowRunEvent) {
        match event {
            WorkflowRunEvent::WorkflowStarted { run_id } => {
                self.value.run_id = run_id.clone();
                self.value.stage = "准备".into();
            }
            WorkflowRunEvent::Child { context, event } => {
                if context.run_id != self.value.run_id {
                    return;
                }
                let next = match event.as_ref() {
                    WorkflowRunEvent::NodeStarted(_) => 1,
                    WorkflowRunEvent::NodeFinished(e)
                        if e.node.status == WorkflowNodeRunStatus::Failed =>
                    {
                        3
                    }
                    WorkflowRunEvent::NodeFinished(_) => 2,
                    _ => return,
                };
                let state = self.chunks.entry(context.chunk_id.clone()).or_default();
                *state = (*state).max(next);
            }
            WorkflowRunEvent::NodeStarted(e) => {
                self.value.stage = match e.node_id.as_str() {
                    "prepare" => "准备与转写",
                    "segments" => "分段",
                    "brief" | "planning" => "分段规划",
                    "merge" => "合并",
                    "recipes" | "assets" => "素材作业",
                    "apply" => "串行应用",
                    "preview" => "独立预览核验",
                    _ => return,
                }
                .into();
            }
            WorkflowRunEvent::NodeFinished(e) => {
                if e.node.status == WorkflowNodeRunStatus::Failed {
                    self.value.stage = "失败".into();
                    self.terminal = true;
                }
                if e.node.node_id == "segments" {
                    self.value.total = e
                        .node
                        .outputs
                        .get("json")
                        .and_then(Value::as_array)
                        .map(Vec::len)
                        .unwrap_or(0);
                }
            }
            WorkflowRunEvent::WorkflowFinished(response) => {
                self.terminal = true;
                self.value.stage = if response.status == WorkflowRunStatus::Succeeded {
                    "预览完成"
                } else {
                    "失败"
                }
                .into();
            }
            _ => {}
        }
    }
    pub(super) fn event_changed(
        &mut self,
        budget: &crate::workflow::budget::VideoEditRunBudget,
    ) -> Option<StreamEvent> {
        let sequence = self.value.sequence;
        let event = self.event(budget);
        let mut current = self.value.clone();
        current.sequence = 0;
        if self.last.as_ref() == Some(&current) {
            self.value.sequence = sequence;
            return None;
        }
        self.last = Some(current);
        Some(event)
    }
    pub(super) fn event(
        &mut self,
        budget: &crate::workflow::budget::VideoEditRunBudget,
    ) -> StreamEvent {
        self.value.sequence += 1;
        self.value.limit = budget.limit();
        self.value.peak = budget.peak();
        self.value.running = self.chunks.values().filter(|&&s| s == 1).count();
        self.value.completed = self.chunks.values().filter(|&&s| s == 2).count();
        self.value.failed = self.chunks.values().filter(|&&s| s == 3).count();
        self.value.queued = self.value.total.saturating_sub(self.chunks.len());
        let jobs = budget.asset_progress();
        self.value.jobs_queued = jobs.queued;
        self.value.jobs_running = jobs.running;
        self.value.jobs_completed = jobs.completed;
        self.value.jobs_failed = jobs.failed;
        let failed =
            self.value.stage == "失败" || self.value.failed > 0 || self.value.jobs_failed > 0;
        let cancelled = self.value.stage == "已取消";
        let status = if cancelled {
            "cancelled"
        } else if failed {
            "failed"
        } else if self.terminal {
            "completed"
        } else {
            "running"
        };
        let call_id = format!("workflow:{}:video-progress", self.value.run_id);
        StreamEvent::Tool(ToolEvent {
            call_id: call_id.clone(),
            name: "workflow_node".into(),
            kind: if self.value.sequence == 1 {
                ToolEventKind::Start
            } else if self.terminal {
                ToolEventKind::Complete
            } else {
                ToolEventKind::Update
            },
            status: if cancelled {
                ToolCallStatus::Cancelled
            } else if failed {
                ToolCallStatus::Failed
            } else if self.terminal {
                ToolCallStatus::Completed
            } else {
                ToolCallStatus::Running
            },
            occurred_ms: crate::session::session::now_ms(),
            payload: json!({"tool_call_id":call_id,"status":status,"summary":"单视频运行，最多五路",
                "metadata":{"title":"云端剪辑进度","video_edit_progress":self.value}}),
        })
    }
}

#[cfg(test)]
#[path = "native_video_progress_tests.rs"]
mod tests;
