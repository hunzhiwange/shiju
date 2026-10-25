//! 定义专家中心各技能页签、异步请求与安装操作的界面状态。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 专家中心的一级内容分区。
pub(crate) enum ExpertHubSection {
    Experts,
    Connectors,
    #[default]
    Skills,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 识矩技能区域的列表页签。
pub(crate) enum SkillTab {
    #[default]
    Marketplace,
    Installed,
    System,
    Mine,
    Install,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 公共技能市场的推荐、单技能与套件页签。
pub(crate) enum PublicSkillTab {
    #[default]
    Recommended,
    SkillHub,
    Suites,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 当前工作区是否满足公共技能安装条件。
pub(crate) enum PublicInstallAvailability {
    Ready,
    NoProject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 公共市场详情指向单个技能或技能套件。
pub(crate) enum PublicDetailTarget {
    Skill(SkillMarketSkillLocatorDto),
    Suite { source: String, slug: String },
}

impl PublicDetailTarget {
    pub(crate) fn source(&self) -> &str {
        match self {
            Self::Skill(locator) => &locator.source,
            Self::Suite { source, .. } => source,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 技能说明的可视化或原始 Markdown 展示模式。
pub(crate) enum PublicDetailMode {
    #[default]
    Visual,
    Markdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// 公共技能详情中的安装进度与完成路径。
pub(crate) enum PublicInstallState {
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
pub(crate) struct PublicDetailState {
    pub(crate) target: PublicDetailTarget,
    pub(crate) generation: u64,
    pub(crate) mode: PublicDetailMode,
    pub(crate) skill_detail: LoadState<SkillMarketSkillDetailDto>,
    pub(crate) skill_file: LoadState<SkillMarketSkillFileDto>,
    pub(crate) suite_detail: LoadState<SkillMarketSkillSetDetailDto>,
    pub(crate) markdown_items: Vec<iced::widget::markdown::Item>,
    pub(crate) install: PublicInstallState,
    pub(crate) install_confirmation_open: bool,
    pub(crate) install_scope: super::SkillInstallScope,
    pub(crate) selected_install_project_path: Option<String>,
    pub(crate) selected_install_targets: Vec<super::SkillInstallTarget>,
}

impl PublicDetailState {
    pub(crate) fn install_is_busy(&self) -> bool {
        matches!(self.install, PublicInstallState::Installing { .. })
    }

    pub(crate) fn select_install_scope(&mut self, scope: super::SkillInstallScope) -> bool {
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

    pub(crate) fn select_install_project_path(&mut self, project_path: String) -> bool {
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
    pub(crate) fn toggle_install_target(&mut self, target: super::SkillInstallTarget) -> bool {
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

/// 关闭代码语法高亮后解析公共内容，避免未知语言导致渲染失败。
pub(crate) fn parse_public_markdown(content: &str) -> Vec<iced::widget::markdown::Item> {
    let content = markdown_without_code_highlighting(content);
    iced::widget::markdown::parse(&content).collect()
}

/// 保留围栏代码内容，但移除开头语言标识以禁用语法高亮。
pub(super) fn markdown_without_code_highlighting(content: &str) -> String {
    let mut normalized = String::with_capacity(content.len());
    let mut open_fence = None;

    for line in content.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let line_ending = &line[body.len()..];

        if let Some((marker, minimum_length)) = open_fence {
            normalized.push_str(line);
            if closing_code_fence(body, marker, minimum_length) {
                open_fence = None;
            }
            continue;
        }

        let Some((indentation, marker, length)) = opening_code_fence(body) else {
            normalized.push_str(line);
            continue;
        };
        normalized.push_str(&body[..indentation]);
        normalized.extend(std::iter::repeat_n(marker, length));
        normalized.push_str(line_ending);
        open_fence = Some((marker, length));
    }

    normalized
}

fn opening_code_fence(line: &str) -> Option<(usize, char, usize)> {
    let indentation = line.bytes().take_while(|byte| *byte == b' ').count();
    if indentation > 3 {
        return None;
    }
    let rest = &line[indentation..];
    let marker = match rest.as_bytes().first() {
        Some(b'`') => '`',
        Some(b'~') => '~',
        _ => return None,
    };
    let length = rest.chars().take_while(|character| *character == marker).count();
    (length >= 3).then_some((indentation, marker, length))
}

fn closing_code_fence(line: &str, marker: char, minimum_length: usize) -> bool {
    let indentation = line.bytes().take_while(|byte| *byte == b' ').count();
    if indentation > 3 {
        return false;
    }
    let rest = &line[indentation..];
    let length = rest.chars().take_while(|character| *character == marker).count();
    length >= minimum_length && rest[length..].trim().is_empty()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 已安装公共技能支持的批量操作。
pub(crate) enum PublicBatchActionKind {
    Enable,
    Disable,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 批量操作启动时冻结的代次、动作和技能选择。
pub(crate) struct PublicBatchActionSnapshot {
    pub(crate) generation: u64,
    pub(crate) kind: PublicBatchActionKind,
    pub(crate) skill_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 批量操作在途集合及逐项成功、失败统计。
pub(crate) struct PublicBatchActionState {
    pub(crate) generation: u64,
    pub(crate) kind: PublicBatchActionKind,
    pub(crate) pending: BTreeSet<String>,
    pub(crate) succeeded: usize,
    pub(crate) failures: BTreeMap<String, String>,
    pub(crate) preserve_selection: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 最近一次批量操作的最终统计。
pub(crate) struct PublicBatchSummary {
    pub(crate) succeeded: usize,
    pub(crate) failed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 公共技能市场已提交与编辑中的查询条件。
pub(crate) struct PublicMarketQuery {
    pub(crate) keyword_draft: String,
    pub(crate) submitted_keyword: String,
    pub(crate) category: Option<String>,
    pub(crate) sort: SkillMarketSort,
    pub(crate) page: u32,
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
pub(crate) struct PublicSkillMarketState {
    pub(crate) tab: PublicSkillTab,
    pub(crate) sources: LoadState<Vec<SkillMarketSourceDto>>,
    pub(crate) current_source: Option<String>,
    pub(crate) recommended: LoadState<Vec<SkillMarketSkillDto>>,
    pub(crate) categories: LoadState<Vec<SkillMarketCategoryDto>>,
    pub(crate) market: LoadState<SkillMarketSkillPageDto>,
    pub(crate) suites: LoadState<SkillMarketSkillSetPageDto>,
    pub(crate) installed: LoadState<Vec<DesktopSkillCatalogEntryDto>>,
    pub(crate) detail: Option<PublicDetailState>,
    pub(crate) installed_open: bool,
    pub(crate) installed_selection: BTreeSet<String>,
    pub(crate) delete_confirmation_open: bool,
    pub(crate) batch_action: Option<PublicBatchActionState>,
    pub(crate) last_batch_summary: Option<PublicBatchSummary>,
    pub(crate) query: PublicMarketQuery,
    pub(crate) suites_page: u32,
    pub(crate) suites_category: Option<String>,
    pub(crate) featured_offset: usize,
    pub(crate) sources_generation: u64,
    pub(crate) recommended_generation: u64,
    pub(crate) categories_generation: u64,
    pub(crate) market_generation: u64,
    pub(crate) suites_generation: u64,
    pub(crate) installed_generation: u64,
    pub(crate) detail_generation: u64,
    pub(crate) install_generation: u64,
    pub(crate) batch_generation: u64,
}

impl PublicSkillMarketState {
    pub(crate) fn install_availability(project_path: Option<&str>) -> PublicInstallAvailability {
        if project_path.is_some_and(|path| !path.trim().is_empty()) {
            PublicInstallAvailability::Ready
        } else {
            PublicInstallAvailability::NoProject
        }
    }

    pub(crate) fn generations(&self) -> [u64; 4] {
        [
            self.recommended_generation,
            self.categories_generation,
            self.market_generation,
            self.suites_generation,
        ]
    }

    /// 仅接受目录中存在的新来源，并使所有来源相关的在途响应失效。
    pub(crate) fn select_source(&mut self, source: &str) -> bool {
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
    pub(crate) fn invalidate_source_content(&mut self) {
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

    pub(crate) fn open_skill_detail(&mut self, locator: SkillMarketSkillLocatorDto) -> u64 {
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

    pub(crate) fn open_suite_detail(&mut self, source: String, slug: String) -> u64 {
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
    pub(crate) fn close_detail(&mut self) {
        self.detail_generation = self.detail_generation.wrapping_add(1);
        self.detail = None;
    }

    pub(crate) fn set_installed_selected(&mut self, skill_id: &str, selected: bool) -> bool {
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

    pub(crate) fn select_all_installed(&mut self) {
        self.installed_selection = match &self.installed {
            LoadState::Loaded(items) => items
                .iter()
                .filter(|item| item.installed && item.source_path.is_some())
                .map(|item| item.id.clone())
                .collect(),
            _ => BTreeSet::new(),
        };
    }

    pub(crate) fn clear_installed_selection(&mut self) {
        self.installed_selection.clear();
    }

    /// 冻结当前选择并启动批量操作；已有操作或空选择时拒绝启动。
    pub(crate) fn start_batch_action(
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

    pub(crate) fn submit_query(&mut self) {
        self.query.submitted_keyword = self.query.keyword_draft.trim().to_string();
        self.query.page = 1;
        self.market_generation = self.market_generation.wrapping_add(1);
        self.suites_generation = self.suites_generation.wrapping_add(1);
        self.market = LoadState::Idle;
        self.suites = LoadState::Idle;
        self.suites_page = 1;
    }

    pub(crate) fn featured(&self, count: usize) -> Vec<&SkillMarketSkillDto> {
        let LoadState::Loaded(skills) = &self.recommended else {
            return Vec::new();
        };
        let count = count.min(skills.len());
        (0..count).map(|index| &skills[(self.featured_offset + index) % skills.len()]).collect()
    }

    pub(crate) fn rotate_featured(&mut self) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 技能版本操作相对于当前版本的方向。
pub(crate) enum SkillVersionDirection {
    Upgrade,
    Rollback,
    Switch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 一次待确认或执行的不可变技能版本操作。
pub(crate) struct SkillVersionAction {
    pub(crate) skill_id: u64,
    pub(crate) version_id: u64,
    pub(crate) target_version: String,
    pub(crate) direction: SkillVersionDirection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 技能安装、卸载、升级解析或版本切换动作。
pub(crate) enum SkillActionKind {
    Install,
    Uninstall,
    ResolveUpgrade { target_version: String },
    Version(SkillVersionAction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 当前在途技能动作及用于结果对齐的代次。
pub(crate) struct SkillActionState {
    pub(crate) skill_id: u64,
    pub(crate) generation: u64,
    pub(crate) kind: SkillActionKind,
}

#[derive(Debug, Clone)]
/// 单个技能详情、说明和版本列表的独立加载状态。
pub(crate) struct SkillDetailState {
    pub(crate) skill_id: u64,
    pub(crate) generation: u64,
    pub(crate) detail: LoadState<super::SkillDetail>,
    pub(crate) readme: LoadState<String>,
    pub(crate) readme_mode: PublicDetailMode,
    pub(crate) readme_items: Vec<iced::widget::markdown::Item>,
    pub(crate) versions: LoadState<Vec<super::SkillVersion>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// 技能列表编辑中与已提交的关键词、标签筛选条件。
pub(crate) struct SkillListFilter {
    pub(crate) keyword_draft: String,
    pub(crate) submitted_keyword: String,
    pub(crate) tag_id_draft: Option<u64>,
    pub(crate) submitted_tag_id: Option<u64>,
}

impl SkillListFilter {
    pub(crate) fn submit(&mut self) {
        self.submitted_keyword = self.keyword_draft.trim().to_string();
        self.submitted_tag_id = self.tag_id_draft;
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// “我的技能”可筛选的发布生命周期状态。
pub(crate) enum MineSkillStatus {
    Draft,
    PendingReview,
    Published,
    Unpublished,
}

impl MineSkillStatus {
    pub(crate) const ALL: [Self; 4] =
        [Self::Draft, Self::PendingReview, Self::Published, Self::Unpublished];
    pub(crate) const DEFAULT: [Self; 3] = [Self::Draft, Self::PendingReview, Self::Published];

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::PendingReview => "pending_review",
            Self::Published => "published",
            Self::Unpublished => "unpublished",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Draft => "草稿",
            Self::PendingReview => "待审核",
            Self::Published => "已发布",
            Self::Unpublished => "已下架",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// “我的技能”当前展开的筛选菜单。
pub(crate) enum MineFilterMenu {
    Tags,
    CompanyTags,
    Statuses,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 识矩包直接安装或卸载的阶段状态。
pub(crate) enum ShijuDirectInstallState {
    #[default]
    Idle,
    Fetching {
        generation: u64,
        package_id: super::ShijuPackageId,
    },
    Installing {
        generation: u64,
        package_id: super::ShijuPackageId,
    },
    Uninstalling {
        generation: u64,
    },
    Installed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 一次手动下载使用的代次和目标平台包。
pub(crate) struct ShijuManualDownloadState {
    pub(crate) generation: u64,
    pub(crate) package_id: super::ShijuPackageId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 在操作开始时冻结的安装目标选择。
pub(crate) struct ShijuSelectionOperation {
    pub(crate) generation: u64,
    pub(crate) selections: Vec<super::SkillInstallSelection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 安装向导的目标选择、在途操作与本地安装探测状态。
pub(crate) struct ShijuInstallGuideState {
    pub(crate) open: bool,
    pub(crate) current_package: Option<super::ShijuPackageId>,
    pub(crate) agent_package: LoadState<super::ShijuPackageRelease>,
    pub(crate) agent_generation: u64,
    pub(crate) manual_download: Option<ShijuManualDownloadState>,
    pub(crate) manual_generation: u64,
    pub(crate) direct_install: ShijuDirectInstallState,
    pub(crate) direct_generation: u64,
    pub(crate) direct_selections: Vec<super::SkillInstallSelection>,
    pub(crate) installation_generation: u64,
    pub(crate) install_scope: super::SkillInstallScope,
    pub(crate) selected_project_path: Option<String>,
    pub(crate) selected_targets: Vec<super::SkillInstallTarget>,
    pub(crate) installations: Vec<super::ShijuTargetInstallation>,
    pub(crate) is_installed: bool,
    pub(crate) installed_version: Option<String>,
    pub(crate) login_sync: Option<ShijuSelectionOperation>,
    pub(crate) login_sync_generation: u64,
    pub(crate) login_sync_in_progress: bool,
    pub(crate) uninstall_confirmation_open: bool,
}

impl ShijuInstallGuideState {
    /// 使全部向导请求代次失效，并清除在途操作但保留用户选择。
    pub(crate) fn reset_requests(&mut self) {
        self.agent_generation = self.agent_generation.wrapping_add(1);
        self.manual_generation = self.manual_generation.wrapping_add(1);
        self.direct_generation = self.direct_generation.wrapping_add(1);
        self.installation_generation = self.installation_generation.wrapping_add(1);
        self.login_sync_generation = self.login_sync_generation.wrapping_add(1);
        self.agent_package = LoadState::Idle;
        self.manual_download = None;
        self.direct_install = ShijuDirectInstallState::Idle;
        self.direct_selections.clear();
        self.login_sync = None;
        self.login_sync_in_progress = false;
        self.uninstall_confirmation_open = false;
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.reset_requests();
    }

    pub(crate) fn start_agent_request(&mut self) -> u64 {
        self.agent_generation = self.agent_generation.wrapping_add(1);
        let generation = self.agent_generation;
        self.agent_package = LoadState::Loading { generation };
        generation
    }

    pub(crate) fn start_manual_download(
        &mut self,
        package_id: super::ShijuPackageId,
    ) -> Option<u64> {
        if self.manual_download.is_some() {
            return None;
        }
        self.manual_generation = self.manual_generation.wrapping_add(1);
        let generation = self.manual_generation;
        self.manual_download = Some(ShijuManualDownloadState { generation, package_id });
        Some(generation)
    }

    /// 冻结当前非空目标选择并进入包获取阶段；忙碌时拒绝重复启动。
    pub(crate) fn start_direct_install(
        &mut self,
        package_id: super::ShijuPackageId,
    ) -> Option<u64> {
        if matches!(
            &self.direct_install,
            ShijuDirectInstallState::Fetching { .. }
                | ShijuDirectInstallState::Installing { .. }
                | ShijuDirectInstallState::Uninstalling { .. }
        ) {
            return None;
        }
        let selections = self.selected_install_selections();
        if selections.is_empty() {
            return None;
        }
        self.direct_generation = self.direct_generation.wrapping_add(1);
        let generation = self.direct_generation;
        self.direct_selections = selections;
        self.direct_install = ShijuDirectInstallState::Fetching { generation, package_id };
        Some(generation)
    }

    pub(crate) fn start_uninstall(&mut self) -> Option<u64> {
        if self.installed_selection_count() == 0
            || matches!(
                &self.direct_install,
                ShijuDirectInstallState::Fetching { .. }
                    | ShijuDirectInstallState::Installing { .. }
                    | ShijuDirectInstallState::Uninstalling { .. }
            )
        {
            return None;
        }
        let selections = self.selected_install_selections();
        self.direct_generation = self.direct_generation.wrapping_add(1);
        let generation = self.direct_generation;
        self.direct_selections = selections;
        self.direct_install = ShijuDirectInstallState::Uninstalling { generation };
        Some(generation)
    }

    pub(crate) fn start_installation_load(&mut self) -> u64 {
        self.installation_generation = self.installation_generation.wrapping_add(1);
        self.installation_generation
    }

    pub(crate) fn start_login_sync(&mut self) -> Option<ShijuSelectionOperation> {
        if self.login_sync.is_some() || self.installed_selection_count() == 0 {
            return None;
        }
        self.login_sync_generation = self.login_sync_generation.wrapping_add(1);
        let operation = ShijuSelectionOperation {
            generation: self.login_sync_generation,
            selections: self.selected_install_selections(),
        };
        self.login_sync = Some(operation.clone());
        self.login_sync_in_progress = true;
        Some(operation)
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.login_sync.is_some()
            || matches!(
                &self.direct_install,
                ShijuDirectInstallState::Fetching { .. }
                    | ShijuDirectInstallState::Installing { .. }
                    | ShijuDirectInstallState::Uninstalling { .. }
            )
    }

    pub(crate) fn select_scope(&mut self, scope: super::SkillInstallScope) -> bool {
        if self.is_busy() || self.install_scope == scope {
            return false;
        }
        self.install_scope = scope;
        self.refresh_selected_installation_status();
        true
    }

    /// 切换目标但至少保留一个选择，任何安装或同步在途时禁止修改。
    pub(crate) fn toggle_target(&mut self, target: super::SkillInstallTarget) -> bool {
        if self.is_busy() {
            return false;
        }
        if let Some(index) = self.selected_targets.iter().position(|selected| *selected == target) {
            if self.selected_targets.len() == 1 {
                return false;
            }
            self.selected_targets.remove(index);
        } else {
            self.selected_targets.push(target);
        }
        self.refresh_selected_installation_status();
        true
    }

    pub(crate) fn selected_install_selections(&self) -> Vec<super::SkillInstallSelection> {
        let mut selections = Vec::with_capacity(self.selected_targets.len());
        for target in &self.selected_targets {
            let selection =
                super::SkillInstallSelection { target: *target, scope: self.install_scope };
            if !selections.contains(&selection) {
                selections.push(selection);
            }
        }
        selections
    }

    pub(crate) fn installation(
        &self,
        scope: super::SkillInstallScope,
        target: super::SkillInstallTarget,
    ) -> Option<&super::ShijuInstallation> {
        self.installations
            .iter()
            .find(|item| item.destination.scope == scope && item.destination.target == target)
            .and_then(|item| item.installation.as_ref())
    }

    pub(crate) fn installed_selection_count(&self) -> usize {
        self.selected_install_selections()
            .iter()
            .filter(|selection| self.installation(selection.scope, selection.target).is_some())
            .count()
    }

    pub(crate) fn set_installations(&mut self, report: super::ShijuInstallationsReport) {
        self.installations = report.targets;
        self.refresh_selected_installation_status();
    }

    pub(crate) fn apply_install_report(&mut self, report: super::ShijuMultiInstallReport) {
        for install in report.installs {
            let installation = super::ShijuInstallation {
                installed_dir: install.destination.installed_dir.clone(),
                version: install.version,
            };
            if let Some(current) = self.installations.iter_mut().find(|current| {
                current.destination.scope == install.destination.scope
                    && current.destination.target == install.destination.target
            }) {
                current.destination = install.destination;
                current.installation = Some(installation);
            } else {
                self.installations.push(super::ShijuTargetInstallation {
                    destination: install.destination,
                    installation: Some(installation),
                });
            }
        }
        self.refresh_selected_installation_status();
    }

    pub(crate) fn apply_uninstall_report(&mut self, report: super::ShijuUninstallReport) {
        for removed in report.targets {
            if !removed.removed {
                continue;
            }
            if let Some(current) = self.installations.iter_mut().find(|current| {
                current.destination.scope == removed.destination.scope
                    && current.destination.target == removed.destination.target
            }) {
                current.installation = None;
            }
        }
        self.refresh_selected_installation_status();
    }

    fn refresh_selected_installation_status(&mut self) {
        let installed = self
            .selected_install_selections()
            .into_iter()
            .filter_map(|selection| {
                self.installation(selection.scope, selection.target)
                    .map(|installation| installation.version.clone())
            })
            .collect::<Vec<_>>();
        self.is_installed = !installed.is_empty();
        self.installed_version = installed.into_iter().flatten().next();
        if !self.is_busy() {
            self.direct_install = if self.is_installed {
                ShijuDirectInstallState::Installed
            } else {
                ShijuDirectInstallState::Idle
            };
        }
    }

    pub(crate) fn set_installation(&mut self, installation: Option<super::ShijuInstallation>) {
        self.is_installed = installation.is_some();
        self.installed_version = installation.and_then(|installation| installation.version);
        self.direct_install = if self.is_installed {
            ShijuDirectInstallState::Installed
        } else {
            ShijuDirectInstallState::Idle
        };
    }
}

impl Default for ShijuInstallGuideState {
    fn default() -> Self {
        Self {
            open: false,
            current_package: super::ShijuPackageId::current(),
            agent_package: LoadState::Idle,
            agent_generation: 0,
            manual_download: None,
            manual_generation: 0,
            direct_install: ShijuDirectInstallState::Idle,
            direct_generation: 0,
            direct_selections: Vec::new(),
            installation_generation: 0,
            install_scope: super::SkillInstallScope::Global,
            selected_project_path: None,
            selected_targets: vec![super::SkillInstallTarget::AgentSkills],
            installations: Vec::new(),
            is_installed: false,
            installed_version: None,
            login_sync: None,
            login_sync_generation: 0,
            login_sync_in_progress: false,
            uninstall_confirmation_open: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 技能服务端操作成功后的本地缓存同步状态。
pub(crate) enum SkillCacheSyncState {
    #[default]
    Idle,
    Syncing {
        generation: u64,
        skill_id: u64,
        refresh_detail: bool,
    },
    Synced {
        skill_id: u64,
    },
    PendingRetry {
        skill_id: u64,
        refresh_detail: bool,
    },
}

impl SkillCacheSyncState {
    pub(crate) fn pending_retry_skill_id(&self) -> Option<u64> {
        match self {
            Self::PendingRetry { skill_id, .. } => Some(*skill_id),
            _ => None,
        }
    }

    pub(crate) fn syncing_skill_id(&self) -> Option<u64> {
        match self {
            Self::Syncing { skill_id, .. } => Some(*skill_id),
            _ => None,
        }
    }

    pub(crate) fn pending_retry_target(&self) -> Option<(u64, bool)> {
        match self {
            Self::PendingRetry { skill_id, refresh_detail } => Some((*skill_id, *refresh_detail)),
            _ => None,
        }
    }

    pub(crate) fn syncing_refreshes_detail(&self, generation: u64, skill_id: u64) -> Option<bool> {
        match self {
            Self::Syncing {
                generation: active_generation,
                skill_id: active_skill_id,
                refresh_detail,
            } if *active_generation == generation && *active_skill_id == skill_id => {
                Some(*refresh_detail)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
/// 专家中心全部页签、筛选器、请求代次与操作状态的聚合。
pub(crate) struct ExpertHubState {
    pub(crate) section: ExpertHubSection,
    pub(crate) skill_tab: SkillTab,
    pub(crate) search_draft: String,
    pub(crate) submitted_search: String,
    pub(crate) market_tag_id: Option<u64>,
    pub(crate) market_company_tag_id: Option<u64>,
    pub(crate) system_filter: SkillListFilter,
    pub(crate) mine_filter: SkillListFilter,
    pub(crate) mine_tag_ids_draft: Vec<u64>,
    pub(crate) submitted_mine_tag_ids: Vec<u64>,
    pub(crate) mine_company_tag_ids_draft: Vec<u64>,
    pub(crate) submitted_mine_company_tag_ids: Vec<u64>,
    pub(crate) mine_statuses_draft: Vec<MineSkillStatus>,
    pub(crate) submitted_mine_statuses: Vec<MineSkillStatus>,
    pub(crate) mine_filter_menu: Option<MineFilterMenu>,
    pub(crate) platform_tag_catalog: LoadState<Vec<super::SkillTag>>,
    pub(crate) company_tag_catalog: LoadState<Vec<super::SkillTag>>,
    pub(crate) system_page: u32,
    pub(crate) mine_page: u32,
    pub(crate) marketplace: LoadState<super::CursorPage<super::MarketplaceSkill>>,
    pub(crate) installed: LoadState<Vec<super::InstalledSkill>>,
    pub(crate) system: LoadState<super::Page<super::SystemSkill>>,
    pub(crate) mine: LoadState<super::Page<super::TenantSkill>>,
    pub(crate) request_generation: u64,
    pub(crate) installed_request_generation: u64,
    pub(crate) tag_catalog_generation: u64,
    pub(crate) detail_generation: u64,
    pub(crate) action_generation: u64,
    pub(crate) cache_sync_generation: u64,
    pub(crate) account_id: Option<String>,
    pub(crate) detail: Option<SkillDetailState>,
    pub(crate) action: Option<SkillActionState>,
    pub(crate) version_confirmation: Option<SkillVersionAction>,
    pub(crate) cache_sync: SkillCacheSyncState,
    pub(crate) package_status: ShijuPackageStatus,
    pub(crate) install_guide: ShijuInstallGuideState,
    pub(crate) public_market: PublicSkillMarketState,
    pub(crate) connectors: super::connectors::ConnectorsState,
    pub(crate) shiju_apps: crate::app::shiju_apps::ShijuAppsState,
}

fn toggle_ordered_ids(selected_ids: &mut Vec<u64>, tag_id: u64, option_order: &[u64]) {
    if selected_ids.contains(&tag_id) {
        selected_ids.retain(|selected| *selected != tag_id);
        return;
    }

    selected_ids.push(tag_id);
    let selected = selected_ids.clone();
    selected_ids.clear();
    for option_id in option_order {
        if selected.contains(option_id) && !selected_ids.contains(option_id) {
            selected_ids.push(*option_id);
        }
    }
    for selected_id in selected {
        if !selected_ids.contains(&selected_id) {
            selected_ids.push(selected_id);
        }
    }
}

impl ExpertHubState {
    pub(crate) fn select_market_tag(&mut self, tag_id: Option<u64>) {
        self.market_tag_id = tag_id;
    }

    pub(crate) fn select_market_company_tag(&mut self, tag_id: Option<u64>) {
        self.market_company_tag_id = tag_id;
    }

    pub(crate) fn toggle_mine_tag(&mut self, tag_id: u64, option_order: &[u64]) {
        toggle_ordered_ids(&mut self.mine_tag_ids_draft, tag_id, option_order);
    }

    pub(crate) fn toggle_mine_company_tag(&mut self, tag_id: u64, option_order: &[u64]) {
        toggle_ordered_ids(&mut self.mine_company_tag_ids_draft, tag_id, option_order);
    }

    pub(crate) fn toggle_mine_status(&mut self, status: MineSkillStatus) {
        if self.mine_statuses_draft.contains(&status) {
            self.mine_statuses_draft.retain(|selected| *selected != status);
        } else {
            self.mine_statuses_draft.push(status);
            self.mine_statuses_draft = MineSkillStatus::ALL
                .into_iter()
                .filter(|option| self.mine_statuses_draft.contains(option))
                .collect();
        }
    }

    pub(crate) fn toggle_mine_filter_menu(&mut self, menu: MineFilterMenu) {
        self.mine_filter_menu = (self.mine_filter_menu != Some(menu)).then_some(menu);
    }

    pub(crate) fn close_mine_filter_menu(&mut self) {
        self.mine_filter_menu = None;
    }

    pub(crate) fn submit_mine_filters(&mut self) {
        self.mine_filter.submit();
        self.submitted_mine_tag_ids.clone_from(&self.mine_tag_ids_draft);
        self.submitted_mine_company_tag_ids.clone_from(&self.mine_company_tag_ids_draft);
        self.submitted_mine_statuses.clone_from(&self.mine_statuses_draft);
        self.close_mine_filter_menu();
    }

    pub(crate) fn reset_mine_filters(&mut self) {
        self.mine_filter.reset();
        self.mine_tag_ids_draft.clear();
        self.submitted_mine_tag_ids.clear();
        self.mine_company_tag_ids_draft.clear();
        self.submitted_mine_company_tag_ids.clear();
        self.mine_statuses_draft = MineSkillStatus::DEFAULT.to_vec();
        self.submitted_mine_statuses = MineSkillStatus::DEFAULT.to_vec();
        self.close_mine_filter_menu();
    }

    pub(crate) fn reset_navigation(&mut self) {
        self.install_guide.close();
        self.section = ExpertHubSection::Skills;
        self.skill_tab = SkillTab::Marketplace;
    }

    /// 账号切换时清除账号范围数据并递增代次，使旧响应全部失效。
    pub(crate) fn clear_for_account(&mut self, account_id: Option<String>) {
        self.search_draft.clear();
        self.submitted_search.clear();
        self.market_tag_id = None;
        self.market_company_tag_id = None;
        self.system_filter.reset();
        self.reset_mine_filters();
        self.system_page = 1;
        self.mine_page = 1;
        self.marketplace = LoadState::Idle;
        self.installed = LoadState::Idle;
        self.system = LoadState::Idle;
        self.mine = LoadState::Idle;
        self.platform_tag_catalog = LoadState::Idle;
        self.company_tag_catalog = LoadState::Idle;
        self.request_generation = self.request_generation.wrapping_add(1);
        self.installed_request_generation = self.installed_request_generation.wrapping_add(1);
        self.tag_catalog_generation = self.tag_catalog_generation.wrapping_add(1);
        self.detail_generation = self.detail_generation.wrapping_add(1);
        self.action_generation = self.action_generation.wrapping_add(1);
        self.cache_sync_generation = self.cache_sync_generation.wrapping_add(1);
        self.account_id = account_id;
        self.detail = None;
        self.action = None;
        self.version_confirmation = None;
        self.cache_sync = SkillCacheSyncState::Idle;
        self.package_status.cancel_refresh();
        self.install_guide.reset_requests();
        self.shiju_apps.clear_for_account(self.account_id.clone());
        self.connectors.clear(self.account_id.clone());
    }

    pub(crate) fn next_request_generation(&mut self) -> u64 {
        self.request_generation = self.request_generation.wrapping_add(1);
        self.request_generation
    }

    pub(crate) fn next_installed_request_generation(&mut self) -> u64 {
        self.installed_request_generation = self.installed_request_generation.wrapping_add(1);
        self.installed_request_generation
    }

    pub(crate) fn next_tag_catalog_generation(&mut self) -> u64 {
        self.tag_catalog_generation = self.tag_catalog_generation.wrapping_add(1);
        self.tag_catalog_generation
    }

    pub(crate) fn next_detail_generation(&mut self) -> u64 {
        self.detail_generation = self.detail_generation.wrapping_add(1);
        self.detail_generation
    }

    /// 为技能启动互斥动作并返回新代次；已有动作时返回 `None`。
    pub(crate) fn start_action(&mut self, skill_id: u64, kind: SkillActionKind) -> Option<u64> {
        if self.action.is_some()
            || self.version_confirmation.is_some()
            || self.cache_sync.syncing_skill_id().is_some()
        {
            return None;
        }
        self.action_generation = self.action_generation.wrapping_add(1);
        let generation = self.action_generation;
        self.action = Some(SkillActionState { skill_id, generation, kind });
        Some(generation)
    }

    pub(crate) fn start_cache_sync(&mut self, skill_id: u64, refresh_detail: bool) -> Option<u64> {
        if self.cache_sync.syncing_skill_id().is_some() {
            return None;
        }
        self.cache_sync_generation = self.cache_sync_generation.wrapping_add(1);
        let generation = self.cache_sync_generation;
        self.cache_sync = SkillCacheSyncState::Syncing { generation, skill_id, refresh_detail };
        Some(generation)
    }

    /// 仅完成与当前代次和技能匹配的缓存同步，迟到结果保持无效。
    pub(crate) fn finish_cache_sync(
        &mut self,
        generation: u64,
        skill_id: u64,
        succeeded: bool,
    ) -> bool {
        if !matches!(
            self.cache_sync,
            SkillCacheSyncState::Syncing {
                generation: active_generation,
                skill_id: active_skill_id,
                ..
            } if active_generation == generation && active_skill_id == skill_id
        ) {
            return false;
        }
        self.cache_sync = if succeeded {
            SkillCacheSyncState::Synced { skill_id }
        } else {
            let refresh_detail = self
                .cache_sync
                .syncing_refreshes_detail(generation, skill_id)
                .expect("active cache sync checked above");
            SkillCacheSyncState::PendingRetry { skill_id, refresh_detail }
        };
        true
    }

    pub(crate) fn request_version_action(&mut self, action: SkillVersionAction) -> Option<u64> {
        if self.action.is_some() || self.version_confirmation.is_some() {
            return None;
        }
        if action.direction == SkillVersionDirection::Rollback {
            self.version_confirmation = Some(action);
            None
        } else {
            let skill_id = action.skill_id;
            self.start_action(skill_id, SkillActionKind::Version(action))
        }
    }

    pub(crate) fn cancel_version_action(&mut self) {
        self.version_confirmation = None;
    }

    pub(crate) fn confirm_version_action(&mut self) -> Option<u64> {
        if self.action.is_some() {
            return None;
        }
        let action = self.version_confirmation.take()?;
        let skill_id = action.skill_id;
        self.start_action(skill_id, SkillActionKind::Version(action))
    }
}

impl Default for ExpertHubState {
    fn default() -> Self {
        Self {
            section: ExpertHubSection::default(),
            skill_tab: SkillTab::default(),
            search_draft: String::new(),
            submitted_search: String::new(),
            market_tag_id: None,
            market_company_tag_id: None,
            system_filter: SkillListFilter::default(),
            mine_filter: SkillListFilter::default(),
            mine_tag_ids_draft: Vec::new(),
            submitted_mine_tag_ids: Vec::new(),
            mine_company_tag_ids_draft: Vec::new(),
            submitted_mine_company_tag_ids: Vec::new(),
            mine_statuses_draft: MineSkillStatus::DEFAULT.to_vec(),
            submitted_mine_statuses: MineSkillStatus::DEFAULT.to_vec(),
            mine_filter_menu: None,
            platform_tag_catalog: LoadState::Idle,
            company_tag_catalog: LoadState::Idle,
            system_page: 1,
            mine_page: 1,
            marketplace: LoadState::Idle,
            installed: LoadState::Idle,
            system: LoadState::Idle,
            mine: LoadState::Idle,
            request_generation: 0,
            installed_request_generation: 0,
            tag_catalog_generation: 0,
            detail_generation: 0,
            action_generation: 0,
            cache_sync_generation: 0,
            account_id: None,
            detail: None,
            action: None,
            version_confirmation: None,
            cache_sync: SkillCacheSyncState::Idle,
            package_status: ShijuPackageStatus::default(),
            install_guide: ShijuInstallGuideState::default(),
            public_market: PublicSkillMarketState::default(),
            connectors: super::connectors::ConnectorsState::default(),
            shiju_apps: crate::app::shiju_apps::ShijuAppsState::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 识矩包服务端版本刷新状态。
pub(crate) enum ShijuPackageRefreshState {
    #[default]
    Idle,
    Refreshing {
        generation: u64,
    },
    Fresh,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// 本地安装版本、服务端版本及刷新进度的组合状态。
pub(crate) struct ShijuPackageStatus {
    pub(crate) local_version: Option<String>,
    pub(crate) server_version: Option<String>,
    pub(crate) checked_at_unix: Option<u64>,
    pub(crate) refresh: ShijuPackageRefreshState,
    refresh_generation: u64,
}

impl ShijuPackageStatus {
    pub(crate) fn from_sources(
        local_version: Option<String>,
        cached: Option<super::PackageVersionCacheEntry>,
    ) -> Self {
        let (server_version, checked_at_unix) = cached.map_or((None, None), |entry| {
            (Some(entry.server_version), Some(entry.checked_at_unix))
        });
        Self {
            local_version,
            server_version,
            checked_at_unix,
            refresh: ShijuPackageRefreshState::Idle,
            refresh_generation: 0,
        }
    }

    pub(crate) fn set_local_version(&mut self, local_version: Option<String>) {
        self.local_version = local_version;
    }

    pub(crate) fn start_refresh(&mut self) -> u64 {
        self.refresh_generation = self.refresh_generation.wrapping_add(1);
        let generation = self.refresh_generation;
        self.refresh = ShijuPackageRefreshState::Refreshing { generation };
        generation
    }

    pub(crate) fn is_refreshing(&self, generation: u64) -> bool {
        matches!(
            self.refresh,
            ShijuPackageRefreshState::Refreshing {
                generation: current,
            } if current == generation
        )
    }

    /// 仅接受当前刷新代次的结果，并据此更新服务端版本或失败状态。
    pub(crate) fn finish_refresh(
        &mut self,
        generation: u64,
        result: Result<super::PackageVersionCacheEntry, String>,
    ) -> bool {
        if !self.is_refreshing(generation) {
            return false;
        }
        match result {
            Ok(entry) => {
                self.server_version = Some(entry.server_version);
                self.checked_at_unix = Some(entry.checked_at_unix);
                self.refresh = ShijuPackageRefreshState::Fresh;
            }
            Err(_) => {
                self.refresh = ShijuPackageRefreshState::Failed;
            }
        }
        true
    }

    pub(crate) fn cancel_refresh(&mut self) {
        self.refresh_generation = self.refresh_generation.wrapping_add(1);
        self.refresh = ShijuPackageRefreshState::Idle;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// 通用异步资源加载状态，加载分支携带请求代次。
pub(crate) enum LoadState<T> {
    #[default]
    Idle,
    NeedsLogin,
    Loading {
        generation: u64,
    },
    Loaded(T),
    Error(String),
}
use std::collections::{BTreeMap, BTreeSet};
use vw_gateway_client::{
    DesktopSkillCatalogEntryDto, SkillMarketCategoryDto, SkillMarketSkillDetailDto,
    SkillMarketSkillDto, SkillMarketSkillFileDto, SkillMarketSkillLocatorDto,
    SkillMarketSkillPageDto, SkillMarketSkillSetDetailDto, SkillMarketSkillSetPageDto,
    SkillMarketSort, SkillMarketSourceDto,
};
