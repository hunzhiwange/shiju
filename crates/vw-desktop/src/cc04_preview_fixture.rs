use serde_json::{Value, json};
use std::time::{Duration, Instant};
use vw_gateway_client::vw_api_types::{
    app_providers::{
        editor_merge::{EditorChoice, EditorOpen, EditorResult, EditorView},
        operations::*,
        preview::{ProviderPreviewRequest, SavedProviderPreviewRequest},
    },
    task::TaskStatus,
};

#[derive(Default)]
pub(crate) struct Fixture {
    revision: u32,
    stale_seen: bool,
    saved: bool,
    operation: Option<Operation>,
}

struct Operation {
    progress: ProviderOperationProgress,
    ready_at: Instant,
    choice: EditorChoice,
}

impl Fixture {
    pub(crate) fn handle(
        &mut self,
        method: &str,
        path: &str,
        payload: &[u8],
    ) -> Option<(&'static str, Value)> {
        let path = path.strip_prefix("/v1/app-providers/")?;
        match (method, path) {
            ("POST", "claude/config/preview") => {
                let request: ProviderPreviewRequest = serde_json::from_slice(payload).unwrap();
                assert_eq!(request.target_host_id, "fixture-host");
                if request.provider_id.as_deref() == Some("provider-1") {
                    return Some((
                        "200 OK",
                        json!({"settings":null, "changes":[], "inactive":[],
                        "unavailable":null, "parseError":{"source":"live.settings","line":5,"column":8}}),
                    ));
                }
                let changes = (1..=28)
                    .map(|i| json!({"path":["中文动态设置", format!("差异字段 {i}")],"kind":"changed"}))
                    .collect::<Vec<_>>();
                let settings = json!({"env":{"ANTHROPIC_API_KEY":"[redacted]"},
                    "alwaysThinkingEnabled":self.saved,"中文配置":true,"版本":self.revision});
                Some((
                    "200 OK",
                    json!({"settings":settings,"changes":changes,
                    "inactive":[["未启用的中文设置"]],"unavailable":null,"parseError":null}),
                ))
            }
            ("POST", "claude/config/saved-preview") => {
                let request: SavedProviderPreviewRequest = serde_json::from_slice(payload).unwrap();
                assert_eq!(request.provider_id, "provider-1");
                Some((
                    "200 OK",
                    json!({"settings":{"已保存的中文设置":true,"env":{"ANTHROPIC_API_KEY":"[redacted]"}}}),
                ))
            }
            ("POST", "claude/provider-0/editor/open") => {
                let request: EditorOpen = serde_json::from_slice(payload).unwrap();
                assert_eq!(request.target_host_id, "fixture-host");
                self.revision += 1;
                Some((
                    "200 OK",
                    json!(EditorView {
                        context: format!("fixture-{}", self.revision),
                        settings: json!({"alwaysThinkingEnabled":self.saved,"中文配置":true}),
                        current: true,
                    }),
                ))
            }
            ("POST", "operations/editor-save") => {
                let request: ProviderOperationRequest = serde_json::from_slice(payload).unwrap();
                let ProviderOperationAction::EditorSave { id, input, .. } = request.action else {
                    panic!("expected editor save");
                };
                assert_eq!(id, "provider-0");
                assert_eq!(input.target_host_id, "fixture-host");
                assert_eq!(input.context, format!("fixture-{}", self.revision));
                assert_eq!(input.resolution.is_some(), input.choice != EditorChoice::Refuse);
                let progress = ProviderOperationProgress {
                    operation_id: "fixture-operation".into(),
                    request_id: request.request_id,
                    status: TaskStatus::Running,
                    phase: ProviderOperationPhase::Preparing,
                    completed: 0,
                    total: 1,
                    cancellation_requested: false,
                    editor_outcome: None,
                    aggregation_outcome: None,
                };
                self.operation = Some(Operation {
                    progress: progress.clone(),
                    ready_at: Instant::now() + Duration::from_secs(3),
                    choice: input.choice,
                });
                Some(("202 Accepted", json!(progress)))
            }
            ("DELETE", "operations/fixture-operation?targetHostId=fixture-host") => {
                let operation = self.operation.as_mut().unwrap();
                operation.progress.cancellation_requested = true;
                operation.progress.status = TaskStatus::Cancelled;
                operation.progress.phase = ProviderOperationPhase::Terminal;
                Some(("200 OK", json!(operation.progress)))
            }
            ("GET", "operations/fixture-operation?targetHostId=fixture-host") => {
                let operation = self.operation.as_mut().unwrap();
                if operation.progress.status == TaskStatus::Running
                    && Instant::now() >= operation.ready_at
                {
                    let outcome = match operation.choice {
                        EditorChoice::Refuse => ProviderEditorOutcome::Reviewed {
                            result: EditorResult {
                                saved: false,
                                conflicts: vec![vec!["中文配置".into(), "动态冲突字段".into()]],
                                resolution: Some(format!("choice-{}", self.revision)),
                            },
                        },
                        _ if !self.stale_seen => {
                            self.stale_seen = true;
                            ProviderEditorOutcome::Rejected { status: 409 }
                        }
                        _ => {
                            self.saved = true;
                            ProviderEditorOutcome::Reviewed {
                                result: EditorResult {
                                    saved: true,
                                    conflicts: vec![],
                                    resolution: None,
                                },
                            }
                        }
                    };
                    operation.progress.status = match outcome {
                        ProviderEditorOutcome::Reviewed { .. } => TaskStatus::Completed,
                        ProviderEditorOutcome::Rejected { .. } => TaskStatus::Failed,
                    };
                    operation.progress.phase = ProviderOperationPhase::Terminal;
                    operation.progress.completed = 1;
                    operation.progress.editor_outcome = Some(outcome);
                }
                Some(("200 OK", json!(operation.progress)))
            }
            _ => None,
        }
    }
}
