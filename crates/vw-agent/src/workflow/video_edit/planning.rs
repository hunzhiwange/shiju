pub use workflow_video_edit::planning::{VisualType, VideoEditEffect, VideoChunkPlan, VideoEditPlan, VideoPlanRequirements, VideoPlanningContext};
pub async fn plan_video_edit(
    runtime: crate::workflow::WorkflowRuntime,
    transcript: &super::transcript::VideoTranscript,
    segments: &[super::segments::VideoSegment],
    requirements: &VideoPlanRequirements,
    context: &VideoPlanningContext,
) -> anyhow::Result<VideoEditPlan> {
    let max_parallel = runtime.video_edit_budget.limit();
    workflow_video_edit::planning::plan_video_edit(max_parallel, move |request| crate::workflow::run_workflow(runtime, request), transcript, segments, requirements, context).await
}
