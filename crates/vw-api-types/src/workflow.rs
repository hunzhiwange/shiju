//! 工作流契约兼容导出。

pub use vw_api_workflow::{
    VIDEO_EDIT_TEMPLATE_UUID,
    is_builtin_workflow_uuid,
    WorkflowRunRequest,
    WorkflowRunStatus,
    WorkflowNodeRunStatus,
    WorkflowPauseDto,
    WorkflowHumanActionDto,
    WorkflowNodeRunDto,
    WorkflowRunResponse,
    WorkflowResumeRequest,
    WorkflowRecordSummary,
    WorkflowRecord,
    WorkflowRecordUpsertBody,
    WorkflowRecordDeleteResponse,
    VideoEditTemplateOptions,
    VideoEditProgress,
};
