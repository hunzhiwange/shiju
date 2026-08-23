//! 协调多个流式控制器在提交节拍中的排空操作。
//!
//! 本模块连接队列分块策略与具体控制器：采集压力、解析计划、应用排空并返回历史单元。
//! 所有排空只从队首进行以维持顺序；调度动画、插入历史等 UI 副作用仍由调用者负责。
//! Orchestrates commit-tick drains across streaming controllers.
//!
//! This module bridges queue-based chunking policy (`chunking`) with the concrete stream
//! controllers (`controller`). Callers provide the current controllers and tick scope; the module
//! computes queue pressure, selects a drain plan, applies it, and returns emitted history cells.
//!
//! The module preserves ordering by draining only from controller queue heads. It does not schedule
//! ticks and it does not mutate UI state directly; callers remain responsible for animation events
//! and history insertion side effects.
//!
//! The main flow is:
//! [`run_commit_tick`] -> [`stream_queue_snapshot`] -> [`QueueSnapshot`] ->
//! [`resolve_chunking_plan`] -> [`ChunkingDecision`]/[`DrainPlan`] ->
//! [`apply_commit_tick_plan`] -> [`CommitTickOutput`].

use std::time::Duration;
use std::time::Instant;

use super::StreamingCell;
use super::chunking::AdaptiveChunkingPolicy;
use super::chunking::ChunkingDecision;
use super::chunking::ChunkingMode;
use super::chunking::DrainPlan;
use super::chunking::QueueSnapshot;
use super::controller::PlanStreamController;
use super::controller::StreamController;

/// 描述提交节拍可在所有模式运行，还是仅在追赶模式运行。
/// Describes whether a commit tick may run in all modes or only in catch-up mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommitTickScope {
    /// 无论当前分块模式如何都运行节拍。
    /// Always run the tick, regardless of current chunking mode.
    AnyMode,
    /// 执行模型转换和策略更新，但仅在 `CatchUp` 模式提交行。
    /// Run model transitions and policy updates, but commit lines only in `CatchUp`.
    CatchUpOnly,
}

/// 描述单次提交节拍产生的结果。
/// Describes what a single commit tick produced.
pub(crate) struct CommitTickOutput {
    /// 本节拍排空流式行所产生的历史单元。
    /// Cells produced by drained stream lines during this tick.
    pub(crate) cells: Vec<StreamingCell>,
    /// 本节拍是否至少存在一个流式控制器。
    /// Whether at least one stream controller was present for this tick.
    pub(crate) has_controller: bool,
    /// 本节拍后所有现有控制器是否均为空闲。
    /// Whether all present controllers were idle after this tick.
    pub(crate) all_idle: bool,
}

impl Default for CommitTickOutput {
    /// 创建表示“未执行提交”的输出。
    ///
    /// 用于节拍被有意抑制的情况，例如作用域仅允许追赶而当前策略并非追赶模式。
    /// Creates an output that represents "no commit performed".
    ///
    /// This is used when a tick is intentionally suppressed, for example when
    /// the scope is [`CommitTickScope::CatchUpOnly`] and policy is not in catch-up mode.
    fn default() -> Self {
        Self { cells: Vec::new(), has_controller: false, all_idle: true }
    }
}

/// 对给定流式控制器运行一次提交节拍。
///
/// 函数先收集队列快照，交给自适应策略生成决定，再把排空计划应用到两个控制器。
/// 传入与当前轮次无关的陈旧引用会误读队列年龄，并可能让策略长时间停留在追赶模式。
/// Runs one commit tick against the provided stream controllers.
///
/// This function collects a [`QueueSnapshot`], asks [`AdaptiveChunkingPolicy`] for a
/// [`ChunkingDecision`], and then applies the resulting [`DrainPlan`] to both controllers.
/// If callers pass stale controller references (for example, references not tied to the
/// current turn), queue age can be misread and the policy may stay in catch-up longer
/// than expected.
pub(crate) fn run_commit_tick(
    policy: &mut AdaptiveChunkingPolicy,
    stream_controller: Option<&mut StreamController>,
    plan_stream_controller: Option<&mut PlanStreamController>,
    scope: CommitTickScope,
    now: Instant,
) -> CommitTickOutput {
    let snapshot =
        stream_queue_snapshot(stream_controller.as_deref(), plan_stream_controller.as_deref(), now);
    let decision = resolve_chunking_plan(policy, snapshot, now);
    if scope == CommitTickScope::CatchUpOnly && decision.mode != ChunkingMode::CatchUp {
        return CommitTickOutput::default();
    }

    apply_commit_tick_plan(decision.drain_plan, stream_controller, plan_stream_controller)
}

/// 构建供分块策略消费的组合队列压力快照。
///
/// 快照累加控制器队列深度，并保留最大最老年龄，以反映当前最延迟的可见行。
/// Builds the combined queue-pressure snapshot consumed by chunking policy.
///
/// The snapshot sums queue depth across controllers and keeps the maximum oldest age
/// so policy decisions reflect the most delayed queued line currently visible.
fn stream_queue_snapshot(
    stream_controller: Option<&StreamController>,
    plan_stream_controller: Option<&PlanStreamController>,
    now: Instant,
) -> QueueSnapshot {
    let mut queued_lines = 0usize;
    let mut oldest_age: Option<Duration> = None;

    if let Some(controller) = stream_controller {
        queued_lines += controller.queued_lines();
        oldest_age = max_duration(oldest_age, controller.oldest_queued_age(now));
    }
    if let Some(controller) = plan_stream_controller {
        queued_lines += controller.queued_lines();
        oldest_age = max_duration(oldest_age, controller.oldest_queued_age(now));
    }

    QueueSnapshot { queued_lines, oldest_age }
}

/// 计算一次策略决定，并在模式转换时发出跟踪日志。
///
/// 转换日志集中在此，调用者可依赖 `run_commit_tick` 提供一致的可观测性。
/// Computes one policy decision and emits a trace log on mode transitions.
///
/// This keeps policy transition logging in one place so callers can rely on
/// [`run_commit_tick`] to provide consistent observability.
fn resolve_chunking_plan(
    policy: &mut AdaptiveChunkingPolicy,
    snapshot: QueueSnapshot,
    now: Instant,
) -> ChunkingDecision {
    let prior_mode = policy.mode();
    let decision = policy.decide(snapshot, now);
    if decision.mode != prior_mode {
        tracing::trace!(
            prior_mode = ?prior_mode,
            new_mode = ?decision.mode,
            queued_lines = snapshot.queued_lines,
            oldest_queued_age_ms = snapshot.oldest_age.map(|age| age.as_millis() as u64),
            entered_catch_up = decision.entered_catch_up,
            "stream chunking mode transition"
        );
    }
    decision
}

/// 将 [`DrainPlan`] 应用于所有可用流式控制器。
///
/// 返回值包含发出的单元，以及排空后所有现有控制器是否均为空闲。
/// Applies a [`DrainPlan`] to all available stream controllers.
///
/// The returned [`CommitTickOutput`] reports emitted cells and whether all
/// present controllers are idle after draining.
fn apply_commit_tick_plan(
    drain_plan: DrainPlan,
    stream_controller: Option<&mut StreamController>,
    plan_stream_controller: Option<&mut PlanStreamController>,
) -> CommitTickOutput {
    let mut output = CommitTickOutput::default();

    if let Some(controller) = stream_controller {
        output.has_controller = true;
        let (cell, is_idle) = drain_stream_controller(controller, drain_plan);
        if let Some(cell) = cell {
            output.cells.push(cell);
        }
        output.all_idle &= is_idle;
    }
    if let Some(controller) = plan_stream_controller {
        output.has_controller = true;
        let (cell, is_idle) = drain_plan_stream_controller(controller, drain_plan);
        if let Some(cell) = cell {
            output.cells.push(cell);
        }
        output.all_idle &= is_idle;
    }

    output
}

/// 对主流式控制器应用一个排空步骤。
///
/// 单行计划映射为一次一行排空，批量计划映射为多行排空，包括即时追赶全部积压。
/// Applies one drain step to the main stream controller.
///
/// [`DrainPlan::Single`] maps to one-line drain; [`DrainPlan::Batch`] maps to
/// multi-line drain (including instant catch-up when policy requests the full
/// queued backlog).
fn drain_stream_controller(
    controller: &mut StreamController,
    drain_plan: DrainPlan,
) -> (Option<StreamingCell>, bool) {
    match drain_plan {
        DrainPlan::Single => controller.on_commit_tick(),
        DrainPlan::Batch(max_lines) => controller.on_commit_tick_batch(max_lines),
    }
}

/// 对计划流控制器应用一个排空步骤。
///
/// 该逻辑镜像主控制器，使两类控制器服从相同分块策略决定。
/// Applies one drain step to the plan stream controller.
///
/// This mirrors [`drain_stream_controller`] so both controller types follow the
/// same chunking policy decisions.
fn drain_plan_stream_controller(
    controller: &mut PlanStreamController,
    drain_plan: DrainPlan,
) -> (Option<StreamingCell>, bool) {
    match drain_plan {
        DrainPlan::Single => controller.on_commit_tick(),
        DrainPlan::Batch(max_lines) => controller.on_commit_tick_batch(max_lines),
    }
}

/// 返回两个可选时长中的较大者。
///
/// 只有一侧存在时保留该值，两侧都缺失时返回空。
/// Returns the greater of two optional durations.
///
/// This helper preserves whichever side is present when only one duration exists.
fn max_duration(lhs: Option<Duration>, rhs: Option<Duration>) -> Option<Duration> {
    match (lhs, rhs) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}
