//! Dify Workflow 后端执行模块。
//!
//! 当前实现面向桌面端已能编辑/加载的 Dify YAML，提供最小可用的本地执行闭环。

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod native;
mod runner;

pub use runner::{
    WorkflowAgentProvider, WorkflowAgentRequest, WorkflowAgentResult, WorkflowAgentTool,
    WorkflowChildContext, WorkflowDocumentExtractor, WorkflowDocumentFile, WorkflowDocumentRequest,
    WorkflowExtractedDocument, WorkflowKnowledgeChunk, WorkflowKnowledgeProvider,
    WorkflowKnowledgeRequest, WorkflowNodeDeltaEvent, WorkflowNodeFinishedEvent,
    WorkflowNodeStartedEvent, WorkflowPauseState, WorkflowPauseStore, WorkflowRunEvent,
    WorkflowRuntime, WorkflowToolProvider, WorkflowToolRequest, WorkflowToolResult,
    resume_workflow, run_workflow, run_workflow_with_events,
};

// 运行器集成测试随实现归 workflow-engine。

#[cfg(not(target_arch = "wasm32"))]
pub mod video_edit;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod store;
#[cfg(not(target_arch = "wasm32"))]
pub mod templates;

#[path = "video_edit/budget.rs"]
pub mod budget;
