//! 轨迹领域模型。

use std::collections::{HashMap, HashSet};

use iced::widget::text_editor;
use serde_json::Value;
pub use super::tab::TrajectoryDetailsTab;



/// 指标是否由当前持久化数据明确提供。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricAvailability<T> {
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
pub enum TrajectoryRecordKind {
    System,
    User,
    Context,
    Assistant,
    Tool,
}

impl TrajectoryRecordKind {
    pub const fn detail_tabs(self) -> &'static [TrajectoryDetailsTab] {
        use TrajectoryDetailsTab as Tab;

        match self {
            Self::System => &[Tab::SystemPrompt, Tab::Tools],
            Self::User | Self::Context => &[Tab::Summary, Tab::Preview, Tab::Raw, Tab::Source],
            Self::Assistant => &[Tab::Summary, Tab::Options, Tab::Usage, Tab::Timing],
            Self::Tool => &[Tab::Summary, Tab::Payload, Tab::Result, Tab::Schema, Tab::Timing],
        }
    }
}

/// 详情检查器可由上层应用直接处理的交互。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrajectoryDetailsAction {
    SelectTab(TrajectoryDetailsTab),
    Close,
}

/// 时间线块宽度的瞬时计算模式。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TrajectoryDurationMode {
    Timed,
    #[default]
    Equal,
}

/// 当前会话的轨迹控制状态；这些字段不持久化到会话。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrajectoryControlsState {
    duration_mode: TrajectoryDurationMode,
    collapsed_turns: HashSet<u32>,
    collapsed_tool_steps: HashSet<u32>,
    session_id: Option<String>,
}

impl TrajectoryControlsState {
    pub const fn duration_mode(&self) -> TrajectoryDurationMode {
        self.duration_mode
    }

    pub fn sync_session(&mut self, session_id: Option<&str>) {
        if self.session_id.as_deref() == session_id {
            return;
        }
        self.collapsed_turns.clear();
        self.collapsed_tool_steps.clear();
        self.session_id = session_id.map(ToString::to_string);
    }

    pub fn for_session(&self, session_id: Option<&str>) -> Self {
        let mut controls = self.clone();
        controls.sync_session(session_id);
        controls
    }

    pub fn toggle_duration_mode(&mut self) {
        self.duration_mode = match self.duration_mode {
            TrajectoryDurationMode::Timed => TrajectoryDurationMode::Equal,
            TrajectoryDurationMode::Equal => TrajectoryDurationMode::Timed,
        };
    }

    pub fn toggle_all_turns(&mut self, projection: &TrajectoryProjection) {
        let turn_indices = projection.turns.iter().map(|turn| turn.index).collect::<Vec<_>>();
        if turn_indices.iter().any(|index| !self.collapsed_turns.contains(index)) {
            self.collapsed_turns.extend(turn_indices);
        } else {
            for index in turn_indices {
                self.collapsed_turns.remove(&index);
            }
        }
    }

    pub fn toggle_turn(&mut self, turn_index: u32) {
        if !self.collapsed_turns.insert(turn_index) {
            self.collapsed_turns.remove(&turn_index);
        }
    }

    pub fn toggle_all_calls(&mut self, projection: &TrajectoryProjection) {
        let step_indices = projection
            .records
            .iter()
            .filter(|record| record.kind == TrajectoryRecordKind::Tool)
            .map(|record| record.step_index)
            .collect::<HashSet<_>>();
        if step_indices.iter().any(|index| !self.collapsed_tool_steps.contains(index)) {
            self.collapsed_tool_steps.extend(step_indices);
        } else {
            for index in step_indices {
                self.collapsed_tool_steps.remove(&index);
            }
        }
    }

    pub fn toggle_tool_step(&mut self, step_index: u32) {
        if !self.collapsed_tool_steps.insert(step_index) {
            self.collapsed_tool_steps.remove(&step_index);
        }
    }

    pub fn reveal_record(&mut self, record: &TrajectoryRecord) {
        if let Some(turn_index) = record.turn_index {
            self.collapsed_turns.remove(&turn_index);
        }
        if record.kind == TrajectoryRecordKind::Tool {
            self.collapsed_tool_steps.remove(&record.step_index);
        }
    }

    pub fn is_turn_collapsed(&self, turn_index: u32) -> bool {
        self.collapsed_turns.contains(&turn_index)
    }

    pub fn is_tool_step_collapsed(&self, step_index: u32) -> bool {
        self.collapsed_tool_steps.contains(&step_index)
    }
}

/// 详情选择和标签状态；关闭时始终清除记录选择。
#[derive(Debug, Clone)]
pub struct TrajectoryDetailsState {
    selected_record_id: Option<String>,
    selected_record_kind: Option<TrajectoryRecordKind>,
    active_tab: TrajectoryDetailsTab,
    collapsed_json_paths: HashMap<TrajectoryDetailsTab, HashSet<String>>,
    values: Vec<String>,
    editors: Vec<Option<text_editor::Content>>,
    context_menu_target: Option<usize>,
    context_menu_position: Option<(f32, f32)>,
}

impl Default for TrajectoryDetailsState {
    fn default() -> Self {
        Self {
            selected_record_id: None,
            selected_record_kind: None,
            active_tab: TrajectoryDetailsTab::Summary,
            collapsed_json_paths: HashMap::new(),
            values: Vec::new(),
            editors: Vec::new(),
            context_menu_target: None,
            context_menu_position: None,
        }
    }
}

impl TrajectoryDetailsState {
    pub fn selected_record_id(&self) -> Option<&str> {
        self.selected_record_id.as_deref()
    }

    pub const fn active_tab(&self) -> TrajectoryDetailsTab {
        self.active_tab
    }

    pub fn json_collapsed_paths(&self) -> HashSet<String> {
        self.collapsed_paths_for(self.active_tab)
    }

    pub fn collapsed_paths_for(&self, tab: TrajectoryDetailsTab) -> HashSet<String> {
        self.collapsed_json_paths.get(&tab).cloned().unwrap_or_default()
    }

    pub fn toggle_json_path(&mut self, path: String) {
        let paths = self.collapsed_json_paths.entry(self.active_tab).or_default();
        if !paths.insert(path.clone()) {
            paths.remove(&path);
        }
        self.close_context_menu();
    }

    pub fn editor(&self, index: usize) -> Option<&text_editor::Content> {
        self.editors.get(index).and_then(Option::as_ref)
    }

    pub fn context_menu_position(&self, index: usize) -> Option<(f32, f32)> {
        (self.context_menu_target == Some(index)).then_some(self.context_menu_position).flatten()
    }

    pub fn sync_values(&mut self, values: Vec<String>) {
        self.editors = values
            .iter()
            .map(|value| {
                crate::text_safety::read_only_editor_content(value)
            })
            .collect();
        self.values = values;
        self.close_context_menu();
    }

    pub fn perform_editor_action(&mut self, index: usize, action: text_editor::Action) {
        self.close_context_menu();
        if let Some(Some(editor)) = self.editors.get_mut(index)
            && !matches!(action, text_editor::Action::Edit(_))
        {
            editor.perform(action);
        }
    }

    pub fn open_context_menu(&mut self, index: usize, x: f32, y: f32) {
        if index < self.values.len() {
            self.context_menu_target = Some(index);
            self.context_menu_position = Some((x, y));
        }
    }

    pub fn close_context_menu(&mut self) {
        self.context_menu_target = None;
        self.context_menu_position = None;
    }

    pub fn context_menu_copy_text(&self) -> Option<String> {
        let index = self.context_menu_target?;
        self.editor(index)
            .and_then(text_editor::Content::selection)
            .filter(|selection| !selection.is_empty())
            .or_else(|| self.values.get(index).cloned())
    }

    pub fn select(&mut self, record_id: impl Into<String>) -> bool {
        self.select_id(record_id.into(), None, TrajectoryDetailsTab::Summary)
    }

    pub fn select_record(&mut self, record: &TrajectoryRecord) -> bool {
        let first_tab =
            record.kind.detail_tabs().first().copied().unwrap_or(TrajectoryDetailsTab::Summary);
        let changed = self.select_id(record.id.clone(), Some(record.kind), first_tab);
        if changed && record.kind == TrajectoryRecordKind::System {
            let collapsed_tools = (1..record.detail.request_tools.len())
                .map(|index| format!("/tools/{index}"))
                .collect::<HashSet<_>>();
            if !collapsed_tools.is_empty() {
                self.collapsed_json_paths.insert(TrajectoryDetailsTab::Tools, collapsed_tools);
            }
        }
        changed
    }

    fn select_id(
        &mut self,
        record_id: String,
        record_kind: Option<TrajectoryRecordKind>,
        first_tab: TrajectoryDetailsTab,
    ) -> bool {
        if self.selected_record_id.as_deref() == Some(record_id.as_str())
            && self.selected_record_kind == record_kind
        {
            return false;
        }
        self.selected_record_id = Some(record_id);
        self.selected_record_kind = record_kind;
        self.active_tab = first_tab;
        self.collapsed_json_paths.clear();
        self.values.clear();
        self.editors.clear();
        self.close_context_menu();
        true
    }

    pub fn apply(&mut self, action: TrajectoryDetailsAction) -> bool {
        match action {
            TrajectoryDetailsAction::SelectTab(tab) => {
                if self.selected_record_id.is_none()
                    || self.active_tab == tab
                    || self
                        .selected_record_kind
                        .is_some_and(|kind| !kind.detail_tabs().contains(&tab))
                {
                    return false;
                }
                self.active_tab = tab;
                self.close_context_menu();
                true
            }
            TrajectoryDetailsAction::Close => {
                let changed = self.selected_record_id.take().is_some()
                    || self.active_tab != TrajectoryDetailsTab::Summary;
                self.selected_record_kind = None;
                self.active_tab = TrajectoryDetailsTab::Summary;
                self.collapsed_json_paths.clear();
                self.values.clear();
                self.editors.clear();
                self.close_context_menu();
                changed
            }
        }
    }
}

/// 投影使用的非负 token 汇总。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrajectoryUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub reasoning_tokens: u64,
}

impl TrajectoryUsage {
    pub const fn new(
        input_tokens: u64,
        output_tokens: u64,
        cached_tokens: u64,
        reasoning_tokens: u64,
    ) -> Self {
        Self { input_tokens, output_tokens, cached_tokens, reasoning_tokens }
    }

    pub fn add_assign(&mut self, other: Self) {
        self.input_tokens = self.input_tokens.saturating_add(other.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(other.output_tokens);
        self.cached_tokens = self.cached_tokens.saturating_add(other.cached_tokens);
        self.reasoning_tokens = self.reasoning_tokens.saturating_add(other.reasoning_tokens);
    }
}

/// 只暴露真实记录的时间指标；工具记录的总耗时与工具耗时来自同一持久化生命周期。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TrajectoryMetrics {
    pub duration_ms: MetricAvailability<u64>,
    pub ttft_ms: MetricAvailability<u64>,
    pub tool_duration_ms: MetricAvailability<u64>,
    pub generation_ms: MetricAvailability<u64>,
    pub throughput_tokens_per_second: MetricAvailability<f64>,
}

/// 一条记录在详情检查器中需要的完整来源数据。
#[derive(Debug, Clone, PartialEq)]
pub struct TrajectoryToolDefinition {
    pub id: String,
    pub description: String,
    pub input_schema: Value,
}

/// 一条记录在详情检查器中需要的完整来源数据。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrajectoryRecordDetail {
    pub input: Option<String>,
    pub output: Option<String>,
    pub reasoning: Option<String>,
    pub tool_name: Option<String>,
    pub call_id: Option<String>,
    pub arguments: Option<String>,
    pub result: Option<String>,
    pub result_step_index: Option<u32>,
    pub source: Option<Value>,
    pub request_options: Option<Value>,
    pub request_tools: Vec<TrajectoryToolDefinition>,
    pub tool_schema: Option<Value>,
    pub raw_fragments: Vec<Value>,
}

/// 可稳定选择和搜索的单条轨迹记录。
#[derive(Debug, Clone, PartialEq)]
pub struct TrajectoryRecord {
    pub id: String,
    pub index: usize,
    pub kind: TrajectoryRecordKind,
    pub step_index: u32,
    pub turn_index: Option<u32>,
    pub preview: String,
    pub started_ms: Option<u64>,
    pub finished_ms: Option<u64>,
    pub usage: MetricAvailability<TrajectoryUsage>,
    pub metrics: TrajectoryMetrics,
    pub detail: TrajectoryRecordDetail,
}

impl TrajectoryRecord {
    /// 在可见摘要与详情文本中执行大小写不敏感搜索。
    pub fn matches_query(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }
        let text_matches = [
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
        .any(|text| text.to_lowercase().contains(&query));
        text_matches
            || [
                self.detail.source.as_ref(),
                self.detail.request_options.as_ref(),
                self.detail.tool_schema.as_ref(),
            ]
            .into_iter()
            .flatten()
            .any(|value| value.to_string().to_lowercase().contains(&query))
            || self.detail.request_tools.iter().any(|tool| {
                tool.id.to_lowercase().contains(&query)
                    || tool.description.to_lowercase().contains(&query)
                    || tool.input_schema.to_string().to_lowercase().contains(&query)
            })
    }
}

/// 一个模型 step 的投影分组。
#[derive(Debug, Clone, PartialEq)]
pub struct TrajectoryStep {
    pub index: u32,
    pub turn_indices: Vec<u32>,
    pub record_ids: Vec<String>,
    pub started_ms: Option<u64>,
    pub finished_ms: Option<u64>,
    pub usage: MetricAvailability<TrajectoryUsage>,
    pub duration_ms: MetricAvailability<u64>,
}

/// 一个用户轮次的稳定记录与 step 分组。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrajectoryTurn {
    pub index: u32,
    pub user_record_id: Option<String>,
    pub step_indices: Vec<u32>,
    pub record_ids: Vec<String>,
}

/// 整个轨迹的汇总数据。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrajectorySummary {
    pub total_steps: usize,
    pub total_turns: usize,
    pub total_records: usize,
    pub system_records: usize,
    pub user_records: usize,
    pub context_records: usize,
    pub assistant_records: usize,
    pub tool_records: usize,
    pub usage: TrajectoryUsage,
    pub elapsed_ms: MetricAvailability<u64>,
}

/// 当前会话状态的完整只读轨迹投影。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrajectoryProjection {
    pub records: Vec<TrajectoryRecord>,
    pub steps: Vec<TrajectoryStep>,
    pub turns: Vec<TrajectoryTurn>,
    pub summary: TrajectorySummary,
}

impl TrajectoryProjection {
    pub fn record(&self, id: &str) -> Option<&TrajectoryRecord> {
        self.records.iter().find(|record| record.id == id)
    }
}
