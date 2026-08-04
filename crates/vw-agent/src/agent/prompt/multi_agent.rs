//! 普通会话、AgentTool 子会话与 task-plan 子会话共享的递归编排策略。

/// 每层父智能体都必须遵循的递归编排策略。
pub(crate) fn recursive_orchestration_policy() -> &'static str {
    "## Recursive Multi-Agent Orchestration\n\n\
     - Default to AgentTool for every task that is not genuinely simple and single-step. Complex, multi-part, exploratory, implementation, review, tasks with independent workstreams, or otherwise non-trivial work should be delegated. When uncertain whether a task is simple, delegate.\n\
     - Identify internal workstreams, ordering dependencies, and write sets before delegation.\n\
     - Launch independent read-only or non-overlapping work with `launch` or `spawn`, dispatching those calls in the same turn so they run in the background and in parallel.\n\
     - Continue useful parent work that does not depend on a launched result instead of waiting immediately.\n\
     - Use `run` only when a sub-agent result is an immediate dependency and the parent has no useful independent work to do first.\n\
     - Run dependency-ordered or overlapping writes serially; do not parallelize work with ordering dependencies or overlapping writes.\n\
     - Retry only safely retryable failures, with a finite attempt limit. Do not retry permission, safety, destructive-operation, or deterministic validation failures without a material change.\n\
     - Use `status` to inspect launched tasks and `wait` for required tasks to reach a terminal state before the final answer. Prefer event-driven `wait` with a bounded timeout over polling.\n\
     - A `wait` timeout means the task may be still running. Do not claim completion, failure, or a result that was not observed; continue independent work, wait again when necessary, or report the precise running state.\n\
     - Use `stop` to cancel owned tasks that are no longer needed, then inspect the resulting terminal state.\n\
     - Respect runtime delegation depth and parallelism limits; a limit refusal is a scheduling boundary, not permission to bypass the runtime.\n\
     - The parent agent remains responsible for checking every required task state, then must verify evidence and synthesize required results before answering."
}

/// 目标验收器对递归后代证据使用的硬门禁。
pub(crate) fn descendant_acceptance_policy() -> &'static str {
    "## AgentTool 后代验收门禁\n\n\
     - 任何后代仍为 active、waiting、queued 或 running 时，目标尚未完成。\n\
     - 任何后代为 failed 或 cancelled，且没有后续成功补救与父会话明确处理时，目标尚未完成。\n\
     - 后代即使 succeeded，其结果仍必须由父会话验证并综合；只有原始后代输出、没有父级核验与综合证据时，目标尚未完成。"
}
