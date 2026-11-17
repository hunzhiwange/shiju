//! 定义专家中心各技能页签、异步请求与安装操作的界面状态。

use gateway_skill_market::{
    SkillMarketCategoryDto,
    SkillMarketSkillDetailDto,
    SkillMarketSkillDto,
    SkillMarketSkillFileDto,
    SkillMarketSkillLocatorDto,
    SkillMarketSkillPageDto,
    SkillMarketSkillSetDetailDto,
    SkillMarketSkillSetPageDto,
    SkillMarketSort,
    SkillMarketSourceDto,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 专家中心的一级内容分区。
pub enum ExpertHubSection {
    Experts,
    Connectors,
    Knowledge,
    #[default]
    Skills,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 识矩技能区域的列表页签。
pub enum SkillTab {
    #[default]
    Marketplace,
    Installed,
    System,
    Mine,
    Install,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 公共技能市场的推荐、单技能与套件页签。
pub enum PublicSkillTab {
    #[default]
    Recommended,
    SkillHub,
    Suites,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 当前工作区是否满足公共技能安装条件。
pub enum PublicInstallAvailability {
    Ready,
    NoProject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 公共市场详情指向单个技能或技能套件。
pub enum PublicDetailTarget {
    Skill(SkillMarketSkillLocatorDto),
    Suite { source: String, slug: String },
}

impl PublicDetailTarget {
    pub fn source(&self) -> &str {
        match self {
            Self::Skill(locator) => &locator.source,
            Self::Suite { source, .. } => source,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 技能说明的可视化或原始 Markdown 展示模式。
pub enum PublicDetailMode {
    #[default]
    Visual,
    Markdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// 公共技能详情中的安装进度与完成路径。
pub enum PublicInstallState {
    #[default]
    Idle,
    Installing {
        generation: u64,
    },
    Installed {
        paths: Vec<String>,
    },
}

#[derive(Debug, Clone)]
/// 公共技能或套件详情及其独立安装选择状态。
pub struct PublicDetailState {
    pub target: PublicDetailTarget,
    pub generation: u64,
    pub mode: PublicDetailMode,
    pub skill_detail: LoadState<SkillMarketSkillDetailDto>,
    pub skill_file: LoadState<SkillMarketSkillFileDto>,
    pub suite_detail: LoadState<SkillMarketSkillSetDetailDto>,
    pub markdown_items: Vec<iced::widget::markdown::Item>,
    pub install: PublicInstallState,
    pub install_confirmation_open: bool,
    pub install_scope: super::SkillInstallScope,
    pub selected_install_project_path: Option<String>,
    pub selected_install_targets: Vec<super::SkillInstallTarget>,
}

impl PublicDetailState {
    pub fn install_is_busy(&self) -> bool {
        matches!(self.install, PublicInstallState::Installing { .. })
    }

    pub fn select_install_scope(&mut self, scope: super::SkillInstallScope) -> bool {
        if !matches!(self.target, PublicDetailTarget::Skill(_))
            || self.install_is_busy()
            || self.install_scope == scope
        {
            return false;
        }
        self.install_scope = scope;
        if matches!(self.install, PublicInstallState::Installed { .. }) {
            self.install = PublicInstallState::Idle;
        }
        true
    }

    pub fn select_install_project_path(&mut self, project_path: String) -> bool {
        if self.install_is_busy()
            || self.selected_install_project_path.as_deref() == Some(project_path.as_str())
        {
            return false;
        }
        self.selected_install_project_path = Some(project_path);
        if matches!(self.install, PublicInstallState::Installed { .. }) {
            self.install = PublicInstallState::Idle;
        }
        true
    }

    /// 切换安装目标，但始终至少保留一个目标且安装中禁止修改。
    pub fn toggle_install_target(&mut self, target: super::SkillInstallTarget) -> bool {
        if !matches!(self.target, PublicDetailTarget::Skill(_)) || self.install_is_busy() {
            return false;
        }
        if let Some(index) =
            self.selected_install_targets.iter().position(|selected| *selected == target)
        {
            if self.selected_install_targets.len() == 1 {
                return false;
            }
            self.selected_install_targets.remove(index);
        } else {
            self.selected_install_targets.push(target);
        }
        if matches!(self.install, PublicInstallState::Installed { .. }) {
            self.install = PublicInstallState::Idle;
        }
        true
    }
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 已安装公共技能支持的批量操作。
pub enum PublicBatchActionKind {
    Enable,
    Disable,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 批量操作启动时冻结的代次、动作和技能选择。
pub struct PublicBatchActionSnapshot {
    pub generation: u64,
    pub kind: PublicBatchActionKind,
    pub skill_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 批量操作在途集合及逐项成功、失败统计。
pub struct PublicBatchActionState {
    pub generation: u64,
    pub kind: PublicBatchActionKind,
    pub pending: BTreeSet<String>,
    pub succeeded: usize,
    pub failures: BTreeMap<String, String>,
    pub preserve_selection: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 最近一次批量操作的最终统计。
pub struct PublicBatchSummary {
    pub succeeded: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 公共技能市场已提交与编辑中的查询条件。
pub struct PublicMarketQuery {
    pub keyword_draft: String,
    pub submitted_keyword: String,
    pub category: Option<String>,
    pub sort: SkillMarketSort,
    pub page: u32,
}

impl Default for PublicMarketQuery {
    fn default() -> Self {
        Self {
            keyword_draft: String::new(),
            submitted_keyword: String::new(),
            category: None,
            sort: SkillMarketSort::Score,
            page: 1,
        }
    }
}

#[derive(Debug, Clone)]
/// 公共技能市场各资源的独立加载状态和请求代次。
pub struct PublicSkillMarketState {
    pub tab: PublicSkillTab,
    pub sources: LoadState<Vec<SkillMarketSourceDto>>,
    pub current_source: Option<String>,
    pub recommended: LoadState<Vec<SkillMarketSkillDto>>,
    pub categories: LoadState<Vec<SkillMarketCategoryDto>>,
    pub market: LoadState<SkillMarketSkillPageDto>,
    pub suites: LoadState<SkillMarketSkillSetPageDto>,
    pub installed: LoadState<Vec<DesktopSkillCatalogEntryDto>>,
    pub detail: Option<PublicDetailState>,
    pub installed_open: bool,
    pub installed_selection: BTreeSet<String>,
    pub delete_confirmation_open: bool,
    pub batch_action: Option<PublicBatchActionState>,
    pub last_batch_summary: Option<PublicBatchSummary>,
    pub query: PublicMarketQuery,
    pub suites_page: u32,
    pub suites_category: Option<String>,
    pub featured_offset: usize,
    pub sources_generation: u64,
    pub recommended_generation: u64,
    pub categories_generation: u64,
    pub market_generation: u64,
    pub suites_generation: u64,
    pub installed_generation: u64,
    pub detail_generation: u64,
    pub install_generation: u64,
    pub batch_generation: u64,
}

impl PublicSkillMarketState {
    pub fn install_availability(project_path: Option<&str>) -> PublicInstallAvailability {
        if project_path.is_some_and(|path| !path.trim().is_empty()) {
            PublicInstallAvailability::Ready
        } else {
            PublicInstallAvailability::NoProject
        }
    }

    pub fn generations(&self) -> [u64; 4] {
        [
            self.recommended_generation,
            self.categories_generation,
            self.market_generation,
            self.suites_generation,
        ]
    }

    /// 仅接受目录中存在的新来源，并使所有来源相关的在途响应失效。
    pub fn select_source(&mut self, source: &str) -> bool {
        let source_exists = matches!(
            &self.sources,
            LoadState::Loaded(sources) if sources.iter().any(|item| item.source == source)
        );
        if !source_exists || self.current_source.as_deref() == Some(source) {
            return false;
        }
        self.current_source = Some(source.to_string());
        self.invalidate_source_content();
        true
    }

    /// 递增各请求代次并清空来源范围内容，防止旧来源响应回填。
    pub fn invalidate_source_content(&mut self) {
        self.recommended_generation = self.recommended_generation.wrapping_add(1);
        self.categories_generation = self.categories_generation.wrapping_add(1);
        self.market_generation = self.market_generation.wrapping_add(1);
        self.suites_generation = self.suites_generation.wrapping_add(1);
        self.detail_generation = self.detail_generation.wrapping_add(1);
        self.recommended = LoadState::Idle;
        self.categories = LoadState::Idle;
        self.market = LoadState::Idle;
        self.suites = LoadState::Idle;
        self.detail = None;
        self.query = PublicMarketQuery::default();
        self.suites_page = 1;
        self.suites_category = None;
        self.featured_offset = 0;
    }

    pub fn open_skill_detail(&mut self, locator: SkillMarketSkillLocatorDto) -> u64 {
        self.detail_generation = self.detail_generation.wrapping_add(1);
        let generation = self.detail_generation;
        self.detail = Some(PublicDetailState {
            target: PublicDetailTarget::Skill(locator),
            generation,
            mode: PublicDetailMode::Visual,
            skill_detail: LoadState::Loading { generation },
            skill_file: LoadState::Loading { generation },
            suite_detail: LoadState::Idle,
            markdown_items: Vec::new(),
            install: PublicInstallState::Idle,
            install_confirmation_open: false,
            install_scope: super::SkillInstallScope::Global,
            selected_install_project_path: None,
            selected_install_targets: vec![super::SkillInstallTarget::VibeWindow],
        });
        generation
    }

    pub fn open_suite_detail(&mut self, source: String, slug: String) -> u64 {
        self.detail_generation = self.detail_generation.wrapping_add(1);
        let generation = self.detail_generation;
        self.detail = Some(PublicDetailState {
            target: PublicDetailTarget::Suite { source, slug },
            generation,
            mode: PublicDetailMode::Visual,
            skill_detail: LoadState::Idle,
            skill_file: LoadState::Idle,
            suite_detail: LoadState::Loading { generation },
            markdown_items: Vec::new(),
            install: PublicInstallState::Idle,
            install_confirmation_open: false,
            install_scope: super::SkillInstallScope::Project,
            selected_install_project_path: None,
            selected_install_targets: vec![super::SkillInstallTarget::VibeWindow],
        });
        generation
    }

    /// 关闭详情并递增代次，使已发出的详情响应自动过期。
    pub fn close_detail(&mut self) {
        self.detail_generation = self.detail_generation.wrapping_add(1);
        self.detail = None;
    }

    pub fn set_installed_selected(&mut self, skill_id: &str, selected: bool) -> bool {
        let manageable = matches!(
            &self.installed,
            LoadState::Loaded(items)
                if items.iter().any(|item| {
                    item.id == skill_id && item.installed && item.source_path.is_some()
                })
        );
        if !manageable {
            return false;
        }
        if selected {
            self.installed_selection.insert(skill_id.to_string())
        } else {
            self.installed_selection.remove(skill_id)
        }
    }

    pub fn select_all_installed(&mut self) {
        self.installed_selection = match &self.installed {
            LoadState::Loaded(items) => items
                .iter()
                .filter(|item| item.installed && item.source_path.is_some())
                .map(|item| item.id.clone())
                .collect(),
            _ => BTreeSet::new(),
        };
    }

    pub fn clear_installed_selection(&mut self) {
        self.installed_selection.clear();
    }

    /// 冻结当前选择并启动批量操作；已有操作或空选择时拒绝启动。
    pub fn start_batch_action(
        &mut self,
        kind: PublicBatchActionKind,
    ) -> Option<PublicBatchActionSnapshot> {
        if self.batch_action.is_some() || self.installed_selection.is_empty() {
            return None;
        }
        self.batch_generation = self.batch_generation.wrapping_add(1);
        let generation = self.batch_generation;
        let skill_ids = self.installed_selection.iter().cloned().collect::<Vec<_>>();
        self.batch_action = Some(PublicBatchActionState {
            generation,
            kind,
            pending: self.installed_selection.clone(),
            succeeded: 0,
            failures: BTreeMap::new(),
            preserve_selection: false,
        });
        self.last_batch_summary = None;
        Some(PublicBatchActionSnapshot { generation, kind, skill_ids })
    }

    pub fn submit_query(&mut self) {
        self.query.submitted_keyword = self.query.keyword_draft.trim().to_string();
        self.query.page = 1;
        self.market_generation = self.market_generation.wrapping_add(1);
        self.suites_generation = self.suites_generation.wrapping_add(1);
        self.market = LoadState::Idle;
        self.suites = LoadState::Idle;
        self.suites_page = 1;
    }

    pub fn featured(&self, count: usize) -> Vec<&SkillMarketSkillDto> {
        let LoadState::Loaded(skills) = &self.recommended else {
            return Vec::new();
        };
        let count = count.min(skills.len());
        (0..count).map(|index| &skills[(self.featured_offset + index) % skills.len()]).collect()
    }

    pub fn rotate_featured(&mut self) {
        if let LoadState::Loaded(skills) = &self.recommended
            && !skills.is_empty()
        {
            self.featured_offset = (self.featured_offset + 1) % skills.len();
        }
    }
}

impl Default for PublicSkillMarketState {
    fn default() -> Self {
        Self {
            tab: PublicSkillTab::default(),
            sources: LoadState::Idle,
            current_source: None,
            recommended: LoadState::Idle,
            categories: LoadState::Idle,
            market: LoadState::Idle,
            suites: LoadState::Idle,
            installed: LoadState::Idle,
            detail: None,
            installed_open: false,
            installed_selection: BTreeSet::new(),
            delete_confirmation_open: false,
            batch_action: None,
            last_batch_summary: None,
            query: PublicMarketQuery::default(),
            suites_page: 1,
            suites_category: None,
            featured_offset: 0,
            sources_generation: 0,
            recommended_generation: 0,
            categories_generation: 0,
            market_generation: 0,
            suites_generation: 0,
            installed_generation: 0,
            detail_generation: 0,
            install_generation: 0,
            batch_generation: 0,
        }
    }
}


use super::LoadState;
use std::collections::{BTreeMap, BTreeSet};
use vw_gateway_client::{DesktopSkillCatalogEntryDto};
