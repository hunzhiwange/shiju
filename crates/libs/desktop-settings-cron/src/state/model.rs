use iced::widget::text_editor;
use super::{CronFrequency, CronHistoryState, CronScheduleDraft};
/// 定时任务设置面板状态
///
/// 管理 Cron 定时任务功能的配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum CronSettingsTab {
    #[default]
    Jobs,
    History,
    Employee,
    Config,
    Scheduler,
    Add,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum CronAddJobType {
    Shell,
    #[default]
    Agent,
}

impl CronAddJobType {
    pub fn as_api_value(self) -> &'static str {
        match self {
            Self::Shell => "shell",
            Self::Agent => "agent",
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum CronAddScheduleKind {
    #[default]
    Cron,
    At,
    Every,
}

impl CronAddScheduleKind {
    pub fn as_api_value(self) -> &'static str {
        match self {
            Self::Cron => "cron",
            Self::At => "at",
            Self::Every => "every",
        }
    }
}


#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CronEditorTab {
    #[default]
    Basic,
    Advanced,
    Delivery,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CronEditorPicker {
    Model,
    Agent,
    Skills,
}

#[derive(Debug, Clone)]
pub enum CronEditorPickerAction {
    Toggle(CronEditorPicker),
    Close,
    Search(String),
    ModelSelected(String),
    AgentSelected { id: String, backend: Option<String> },
}

#[derive(Debug, Clone)]
pub struct CronJobDraft {
    pub schedule_draft: Option<CronScheduleDraft>,
    pub schedule_revision: u64,
    pub schedule_error: Option<String>,
    pub preview_loading: bool,
    pub preview: Option<Result<gateway_cron::CronPreviewResponse, String>>,
    pub submitting: bool,
    pub name: String,
    pub job_type: CronAddJobType,
    pub schedule_kind: CronAddScheduleKind,
    pub schedule: String,
    pub frequency: CronFrequency,
    pub at: String,
    pub every_ms: String,
    pub command: String,
    pub command_editor: text_editor::Content,
    pub prompt: String,
    pub skills: Vec<String>,
    pub editor_tab: CronEditorTab,
    pub editor_picker: Option<CronEditorPicker>,
    pub editor_picker_query: String,
    pub prompt_editor: text_editor::Content,
    pub session_target: String,
    pub agent: String,
    pub acp_agent: String,
    pub project_path: String,
    pub model_provider: String,
    pub model: String,
    pub wake: bool,
    pub fallbacks: String,
    pub full_access: bool,
    pub full_access_confirmation: bool,
    pub task_pool: bool,
    pub delivery_enabled: bool,
    pub delivery_channel: String,
    pub delivery_to: String,
    pub delivery_best_effort: bool,
    pub delete_after_run: bool,
}

impl Default for CronJobDraft {
    fn default() -> Self {
        Self {
            schedule_draft: None,
            schedule_revision: 0,
            schedule_error: None,
            preview_loading: false,
            preview: None,
            submitting: false,
            name: String::new(),
            job_type: CronAddJobType::default(),
            schedule_kind: CronAddScheduleKind::default(),
            schedule: CronFrequency::Daily.default_expression().to_string(),
            frequency: CronFrequency::Daily,
            at: String::new(),
            every_ms: String::new(),
            command: String::new(),
            command_editor: text_editor::Content::new(),
            prompt: String::new(),
            skills: Vec::new(),
            editor_tab: CronEditorTab::Basic,
            editor_picker: None,
            editor_picker_query: String::new(),
            prompt_editor: text_editor::Content::new(),
            session_target: "isolated".to_string(),
            agent: "main".to_string(),
            acp_agent: String::new(),
            project_path: String::new(),
            model_provider: String::new(),
            model: String::new(),
            wake: false,
            fallbacks: String::new(),
            full_access: false,
            full_access_confirmation: false,
            task_pool: false,
            delivery_enabled: false,
            delivery_channel: String::new(),
            delivery_to: String::new(),
            delivery_best_effort: true,
            delete_after_run: false,
        }
    }
}

/// 当前列表分组，只影响内存中的展示和选择范围。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CronListFilter {
    #[default]
    All,
    Enabled,
    Paused,
}

impl CronListFilter {
    pub fn matches(self, job: &gateway_cron::CronJobDto) -> bool {
        match self {
            Self::All => true,
            Self::Enabled => job.enabled,
            Self::Paused => !job.enabled,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CronSettingsState {
    pub show_add_modal: bool,
    pub show_cron_templates: bool,
    pub skill_catalog: Vec<crate::system_settings_cron::SkillChoice>,
    pub skill_catalog_request: Option<iced::task::Handle>,
    pub skill_catalog_revision: u64,
    pub skill_catalog_failed: bool,
    pub date_time_picker:
        Option<crate::system_settings_cron::date_time::DateTimePicker>,
    /// 是否启用定时任务
    pub enabled: bool,
    /// 最大运行历史记录数
    pub max_run_history: u32,
    /// 当前 Cron 设置页签。
    pub active_tab: CronSettingsTab,
    pub history: CronHistoryState,
    /// 当前任务列表加载状态。
    pub jobs_loading: bool,
    /// 当前任务列表。
    pub jobs: Vec<gateway_cron::CronJobDto>,
    /// 当前列表分组。
    pub list_filter: CronListFilter,
    /// 是否显示批量管理控件。
    pub batch_mode: bool,
    /// 被批量操作选中的任务 ID。
    pub selected_job_ids: Vec<String>,
    /// 当前展开详情的任务 ID。
    pub expanded_job_ids: Vec<String>,
    /// 当前正在编辑的任务 ID。
    pub editing_job_id: Option<String>,
    /// 编辑表单草稿。
    pub edit_draft: CronJobDraft,
    /// 新增表单草稿。
    pub add_draft: CronJobDraft,
    /// 正在查看历史记录的任务 ID。
    pub runs_modal_job_id: Option<String>,
    /// 历史记录加载状态。
    pub runs_modal_loading: bool,
    /// 历史记录加载错误。
    pub runs_modal_error: Option<String>,
    /// 当前弹窗展示的历史记录。
    pub runs_modal: Vec<gateway_cron::CronRunDto>,
    /// 当前弹窗中用于选择复制的完整历史文本。
    pub runs_modal_editor: text_editor::Content,
    /// 是否显示帮助对话框
    pub show_help_modal: bool,
    /// 保存错误信息
    pub save_error: Option<String>,
    pub save_error_translation: Option<desktop_i18n::format::Message>,
    /// 最近一次任务操作提示。
    pub action_status: Option<String>,
}

impl CronSettingsState {
    pub fn visible_jobs(&self) -> impl Iterator<Item = &gateway_cron::CronJobDto> {
        self.jobs.iter().filter(|job| self.list_filter.matches(job))
    }

    /// 计数始终基于完整列表，不受当前分组影响。
    pub fn job_count(&self, filter: CronListFilter) -> usize {
        self.jobs.iter().filter(|job| filter.matches(job)).count()
    }

    pub fn prune_selected_jobs(&mut self) {
        self.selected_job_ids.retain(|id| {
            self.jobs.iter().any(|job| job.id == *id && self.list_filter.matches(job))
        });
    }
}

impl Default for CronSettingsState {
    fn default() -> Self {
        Self {
            show_add_modal: false,
            show_cron_templates: false,
            skill_catalog: Vec::new(),
            skill_catalog_request: None,
            skill_catalog_revision: 0,
            skill_catalog_failed: false,
            date_time_picker: None,
            enabled: true,
            max_run_history: 50,
            active_tab: CronSettingsTab::default(),
            history: CronHistoryState::default(),
            jobs_loading: false,
            jobs: Vec::new(),
            list_filter: CronListFilter::default(),
            batch_mode: false,
            selected_job_ids: Vec::new(),
            expanded_job_ids: Vec::new(),
            editing_job_id: None,
            edit_draft: CronJobDraft::default(),
            add_draft: CronJobDraft::default(),
            runs_modal_job_id: None,
            runs_modal_loading: false,
            runs_modal_error: None,
            runs_modal: Vec::new(),
            runs_modal_editor: text_editor::Content::new(),
            show_help_modal: false,
            save_error: None,
            save_error_translation: None,
            action_status: None,
        }
    }
}


/// References use the same names the gateway resolves from user prompts.
pub fn valid_skill_reference(id: &str) -> bool {
    !id.is_empty()
        && id.chars().all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_' | ':' | '@' | '.'))
}

impl CronJobDraft {
    /// Returns the typed schedule, including schedules saved in the legacy fields.
    pub fn effective_schedule(&self) -> CronScheduleDraft {
        self.schedule_draft.clone().unwrap_or_else(|| {
            let request = gateway_cron::CronPreviewRequest {
                schedule_kind: self.schedule_kind.as_api_value().into(),
                schedule: Some(self.schedule.clone()),
                at: Some(self.at.clone()),
                every_ms: self.every_ms.parse().ok(),
                ..Default::default()
            };
            CronScheduleDraft::from_request(&request).unwrap_or_default()
        })
    }

    pub fn load_prompt(&mut self, saved: &str) {
        self.skills.clear();
        self.prompt = saved.to_owned();
        if let Some((references, prompt)) = saved.split_once("\n\n") {
            let skills = references
                .lines()
                .map(|line| line.strip_prefix('$').filter(|id| valid_skill_reference(id)))
                .collect::<Option<Vec<_>>>();
            if let Some(skills) = skills.filter(|ids| !ids.is_empty()) {
                self.skills = skills.into_iter().map(str::to_owned).collect();
                self.prompt = prompt.to_owned();
            }
        }
        self.prompt_editor = text_editor::Content::with_text(&self.prompt);
    }

    pub(crate) fn prompt_for_gateway(&self) -> String {
        if self.job_type != CronAddJobType::Agent || self.skills.is_empty() {
            return self.prompt.clone();
        }
        let references = self.skills.iter().map(|id| format!("${id}")).collect::<Vec<_>>();
        format!("{}\n\n{}", references.join("\n"), self.prompt)
    }

    /// 切换类型时丢弃上一类型的目标，避免把用户 ID 当作群引用。
    pub(crate) fn select_delivery_channel(&mut self, channel: String) {
        if self.delivery_channel != channel {
            self.delivery_to =
                if channel == "wecom_webhook" { "default".into() } else { String::new() };
            self.delivery_channel = channel;
        }
    }
}
