//! 当前工作区运行记录的筛选和请求生命周期。
use vw_gateway_client::{CronHistoryResponse, CronHistoryRunDto, CronJobDto};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CronHistoryStatus {
    #[default]
    All,
    Running,
    Success,
    Failed,
}

impl CronHistoryStatus {
    fn matches(self, status: &str) -> bool {
        match self {
            Self::All => true,
            Self::Running => status == "running",
            Self::Success => status == "ok",
            Self::Failed => status == "error",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CronHistoryLocation {
    #[default]
    All,
    Local,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CronHistoryState {
    pub runs: Vec<CronHistoryRunDto>,
    pub loading: bool,
    /// 有错误时已有行仍可查看，但不是最新数据。
    pub error: Option<String>,
    pub search: String,
    pub search_open: bool,
    /// Stable single-run key; resolved against current rows on every view.
    pub selected_run: Option<String>,
    pub status_filter: CronHistoryStatus,
    pub location_filter: CronHistoryLocation,
    pub selected_job_id: Option<String>,
    pub task_search: String,
    pub task_picker_open: bool,
    pub generation: u64,
    pub finished_limit: usize,
    pub truncated: bool,
    visible: bool,
    workspace: Option<String>,
}

fn name_matches(name: &str, search: &str) -> bool {
    name.to_lowercase().contains(&search.trim().to_lowercase())
}

impl CronHistoryState {
    pub(crate) fn display_name(run: &CronHistoryRunDto) -> &str {
        run.name.as_deref().map(str::trim).filter(|name| !name.is_empty()).unwrap_or(&run.job_id)
    }

    pub(crate) fn visible_runs(&self) -> impl Iterator<Item = &CronHistoryRunDto> {
        self.runs.iter().filter(|run| {
            self.status_filter.matches(&run.status)
                && name_matches(Self::display_name(run), &self.search)
                && self.selected_job_id.as_ref().is_none_or(|id| *id == run.job_id)
                && matches!(
                    self.location_filter,
                    CronHistoryLocation::All | CronHistoryLocation::Local
                )
        })
    }

    /// 候选来自全部任务，未运行过的任务同样可选；重名显示 ID 提示。
    pub(crate) fn task_candidates<'a>(&'a self, jobs: &'a [CronJobDto]) -> Vec<(&'a str, String)> {
        let name = |job: &'a CronJobDto| {
            job.name.as_deref().map(str::trim).filter(|name| !name.is_empty()).unwrap_or(&job.id)
        };
        jobs.iter()
            .filter(|job| name_matches(name(job), &self.task_search))
            .map(|job| {
                let label = if jobs.iter().filter(|other| name(other) == name(job)).count() > 1 {
                    let short: String = job.id.chars().take(8).collect();
                    format!("{} ({short})", name(job))
                } else {
                    name(job).to_string()
                };
                (job.id.as_str(), label)
            })
            .collect()
    }

    /// 返回是否需要为新可见上下文立即加载。离页失效所有在途响应。
    pub(crate) fn sync_context(&mut self, visible: bool, workspace: Option<String>) -> bool {
        if self.visible == visible && self.workspace == workspace {
            return false;
        }
        let workspace_changed = self.workspace != workspace;
        self.generation = self.generation.wrapping_add(1);
        self.visible = visible;
        self.workspace = workspace;
        self.loading = false;
        self.task_picker_open = false;
        self.selected_run = None;
        if workspace_changed {
            self.runs.clear();
            self.error = None;
            self.selected_job_id = None;
            self.finished_limit = 0;
            self.truncated = false;
        }
        visible
    }

    pub(crate) fn begin_request(&mut self) -> Option<u64> {
        if !self.visible || self.loading {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        self.loading = true;
        Some(self.generation)
    }

    pub(crate) fn accepts(&self, generation: u64) -> bool {
        self.visible && self.loading && self.generation == generation
    }

    pub(crate) fn apply_response(&mut self, response: CronHistoryResponse, jobs: &[CronJobDto]) {
        self.runs = response.runs;
        self.finished_limit = response.finished_limit;
        self.truncated = response.truncated;
        self.loading = false;
        self.error = None;
        if self.selected_job_id.as_ref().is_some_and(|id| !jobs.iter().any(|job| job.id == *id)) {
            self.selected_job_id = None;
        }
    }
}

#[cfg(test)]
#[path = "cron_history_tests.rs"]
mod tests;

impl std::fmt::Display for CronHistoryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::All => "全部状态",
            Self::Running => "运行中",
            Self::Success => "成功",
            Self::Failed => "失败",
        })
    }
}

impl std::fmt::Display for CronHistoryLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::All => "全部类型",
            Self::Local => "本地",
        })
    }
}
