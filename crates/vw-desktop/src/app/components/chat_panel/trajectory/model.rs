//! 轨迹领域模型。

use serde_json::Value;

use crate::app::state::TrajectoryDetailsTab;

/// 指标是否由当前持久化数据明确提供。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum MetricAvailability<T> {
    Available(T),
    Unavailable,
}

impl<T> Default for MetricAvailability<T> {
    fn default() -> Self {
        Self::Unavailable
    }
}

/// 轨迹记录的封闭类别集合。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrajectoryRecordKind {
    System,
    User,
    Assistant,
    Tool,
}

/// 详情检查器可由上层应用直接处理的交互。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrajectoryDetailsAction {
    SelectTab(TrajectoryDetailsTab),
    Close,
}

/// 详情选择和标签状态；关闭时始终清除记录选择。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TrajectoryDetailsState {
    selected_record_id: Option<String>,
    active_tab: TrajectoryDetailsTab,
}

impl TrajectoryDetailsState {
    pub(crate) fn selected_record_id(&self) -> Option<&str> {
        self.selected_record_id.as_deref()
    }

    pub(crate) const fn active_tab(&self) -> TrajectoryDetailsTab {
        self.active_tab
    }

    pub(crate) fn select(&mut self, record_id: impl Into<String>) -> bool {
        let record_id = record_id.into();
        if self.selected_record_id.as_deref() == Some(record_id.as_str()) {
            return false;
        }
        self.selected_record_id = Some(record_id);
        self.active_tab = TrajectoryDetailsTab::Summary;
        true
    }

    pub(crate) fn apply(&mut self, action: TrajectoryDetailsAction) -> bool {
        match action {
            TrajectoryDetailsAction::SelectTab(tab) => {
                if self.selected_record_id.is_none() || self.active_tab == tab {
                    return false;
                }
                self.active_tab = tab;
                true
            }
            TrajectoryDetailsAction::Close => {
                let changed = self.selected_record_id.take().is_some()
                    || self.active_tab != TrajectoryDetailsTab::Summary;
                self.active_tab = TrajectoryDetailsTab::Summary;
                changed
            }
        }
    }
}

/// 投影使用的非负 token 汇总。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TrajectoryUsage {
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
    pub(crate) cached_tokens: u64,
    pub(crate) reasoning_tokens: u64,
}

impl TrajectoryUsage {
    pub(crate) const fn new(
        input_tokens: u64,
        output_tokens: u64,
        cached_tokens: u64,
        reasoning_tokens: u64,
    ) -> Self {
        Self { input_tokens, output_tokens, cached_tokens, reasoning_tokens }
    }

    pub(crate) fn add_assign(&mut self, other: Self) {
        self.input_tokens = self.input_tokens.saturating_add(other.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(other.output_tokens);
        self.cached_tokens = self.cached_tokens.saturating_add(other.cached_tokens);
        self.reasoning_tokens = self.reasoning_tokens.saturating_add(other.reasoning_tokens);
    }
}

/// 只暴露真实记录的时间指标；其余指标保持明确不可用。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct TrajectoryMetrics {
    pub(crate) duration_ms: MetricAvailability<u64>,
    pub(crate) ttft_ms: MetricAvailability<u64>,
    pub(crate) tool_duration_ms: MetricAvailability<u64>,
    pub(crate) generation_ms: MetricAvailability<u64>,
    pub(crate) throughput_tokens_per_second: MetricAvailability<f64>,
}

/// 一条记录在详情检查器中需要的完整来源数据。
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TrajectoryRecordDetail {
    pub(crate) input: Option<String>,
    pub(crate) output: Option<String>,
    pub(crate) reasoning: Option<String>,
    pub(crate) tool_name: Option<String>,
    pub(crate) call_id: Option<String>,
    pub(crate) arguments: Option<String>,
    pub(crate) result: Option<String>,
    pub(crate) result_step_index: Option<u32>,
    pub(crate) raw_fragments: Vec<Value>,
}

/// 可稳定选择和搜索的单条轨迹记录。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TrajectoryRecord {
    pub(crate) id: String,
    pub(crate) index: usize,
    pub(crate) kind: TrajectoryRecordKind,
    pub(crate) step_index: u32,
    pub(crate) turn_index: Option<u32>,
    pub(crate) preview: String,
    pub(crate) started_ms: Option<u64>,
    pub(crate) finished_ms: Option<u64>,
    pub(crate) usage: MetricAvailability<TrajectoryUsage>,
    pub(crate) metrics: TrajectoryMetrics,
    pub(crate) detail: TrajectoryRecordDetail,
}

impl TrajectoryRecord {
    /// 在可见摘要与详情文本中执行大小写不敏感搜索。
    pub(crate) fn matches_query(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }
        [
            Some(self.preview.as_str()),
            self.detail.input.as_deref(),
            self.detail.output.as_deref(),
            self.detail.reasoning.as_deref(),
            self.detail.tool_name.as_deref(),
            self.detail.call_id.as_deref(),
            self.detail.arguments.as_deref(),
            self.detail.result.as_deref(),
        ]
        .into_iter()
        .flatten()
        .any(|text| text.to_lowercase().contains(&query))
    }
}

/// 一个模型 step 的投影分组。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TrajectoryStep {
    pub(crate) index: u32,
    pub(crate) turn_indices: Vec<u32>,
    pub(crate) record_ids: Vec<String>,
    pub(crate) started_ms: Option<u64>,
    pub(crate) finished_ms: Option<u64>,
    pub(crate) usage: MetricAvailability<TrajectoryUsage>,
    pub(crate) duration_ms: MetricAvailability<u64>,
}

/// 一个用户轮次的稳定记录与 step 分组。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrajectoryTurn {
    pub(crate) index: u32,
    pub(crate) user_record_id: Option<String>,
    pub(crate) step_indices: Vec<u32>,
    pub(crate) record_ids: Vec<String>,
}

/// 整个轨迹的汇总数据。
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TrajectorySummary {
    pub(crate) total_steps: usize,
    pub(crate) total_turns: usize,
    pub(crate) total_records: usize,
    pub(crate) system_records: usize,
    pub(crate) user_records: usize,
    pub(crate) assistant_records: usize,
    pub(crate) tool_records: usize,
    pub(crate) usage: TrajectoryUsage,
    pub(crate) elapsed_ms: MetricAvailability<u64>,
}

/// 当前会话状态的完整只读轨迹投影。
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TrajectoryProjection {
    pub(crate) records: Vec<TrajectoryRecord>,
    pub(crate) steps: Vec<TrajectoryStep>,
    pub(crate) turns: Vec<TrajectoryTurn>,
    pub(crate) summary: TrajectorySummary,
}

impl TrajectoryProjection {
    pub(crate) fn record(&self, id: &str) -> Option<&TrajectoryRecord> {
        self.records.iter().find(|record| record.id == id)
    }
}
