//! Dify Workflow 后端执行模块。
//!
//! 当前实现面向桌面端已能编辑/加载的 Dify YAML，提供最小可用的本地执行闭环。

mod code_runner;
mod code_runner_http;
mod conditions;
mod model;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod native;
mod runner;
mod template;
mod variables;

pub use runner::{
    WorkflowAgentProvider, WorkflowAgentRequest, WorkflowAgentResult, WorkflowAgentTool,
    WorkflowDocumentExtractor, WorkflowDocumentFile, WorkflowDocumentRequest,
    WorkflowExtractedDocument, WorkflowKnowledgeChunk, WorkflowKnowledgeProvider,
    WorkflowKnowledgeRequest, WorkflowNodeDeltaEvent, WorkflowNodeFinishedEvent,
    WorkflowNodeStartedEvent, WorkflowPauseState, WorkflowPauseStore, WorkflowRunEvent,
    WorkflowRuntime, WorkflowToolProvider, WorkflowToolRequest, WorkflowToolResult,
    resume_workflow, run_workflow, run_workflow_with_events,
};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod video_edit;
