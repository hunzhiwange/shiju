use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 技能版本操作相对于当前版本的方向。
pub enum SkillVersionDirection {
    Upgrade,
    Rollback,
    Switch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 一次待确认或执行的不可变技能版本操作。
pub struct SkillVersionAction {
    pub skill_id: u64,
    pub version_id: u64,
    pub target_version: String,
    pub direction: SkillVersionDirection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 技能安装、卸载、升级解析或版本切换动作。
pub enum SkillActionKind {
    Install,
    Uninstall,
    ResolveUpgrade { target_version: String },
    Version(SkillVersionAction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 当前在途技能动作及用于结果对齐的代次。
pub struct SkillActionState {
    pub skill_id: u64,
    pub generation: u64,
    pub kind: SkillActionKind,
}

#[derive(Debug, Clone)]
/// 单个技能详情、说明和版本列表的独立加载状态。
pub struct SkillDetailState {
    pub skill_id: u64,
    pub generation: u64,
    pub detail: LoadState<super::SkillDetail>,
    pub readme: LoadState<String>,
    pub readme_mode: PublicDetailMode,
    pub readme_items: Vec<iced::widget::markdown::Item>,
    pub versions: LoadState<Vec<super::SkillVersion>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// 技能列表编辑中与已提交的关键词、标签筛选条件。
pub struct SkillListFilter {
    pub keyword_draft: String,
    pub submitted_keyword: String,
    pub tag_id_draft: Option<u64>,
    pub submitted_tag_id: Option<u64>,
}

impl SkillListFilter {
    pub fn submit(&mut self) {
        self.submitted_keyword = self.keyword_draft.trim().to_string();
        self.submitted_tag_id = self.tag_id_draft;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// “我的技能”可筛选的发布生命周期状态。
pub enum MineSkillStatus {
    Draft,
    PendingReview,
    Published,
    Unpublished,
}

impl MineSkillStatus {
    pub const ALL: [Self; 4] =
        [Self::Draft, Self::PendingReview, Self::Published, Self::Unpublished];
    pub const DEFAULT: [Self; 3] = [Self::Draft, Self::PendingReview, Self::Published];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::PendingReview => "pending_review",
            Self::Published => "published",
            Self::Unpublished => "unpublished",
        }
    }

    pub const fn label(self) -> &'static str {
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
pub enum MineFilterMenu {
    Tags,
    CompanyTags,
    Statuses,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 技能服务端操作成功后的本地缓存同步状态。
pub enum SkillCacheSyncState {
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
    pub fn pending_retry_skill_id(&self) -> Option<u64> {
        match self {
            Self::PendingRetry { skill_id, .. } => Some(*skill_id),
            _ => None,
        }
    }

    pub fn syncing_skill_id(&self) -> Option<u64> {
        match self {
            Self::Syncing { skill_id, .. } => Some(*skill_id),
            _ => None,
        }
    }

    pub fn pending_retry_target(&self) -> Option<(u64, bool)> {
        match self {
            Self::PendingRetry { skill_id, refresh_detail } => Some((*skill_id, *refresh_detail)),
            _ => None,
        }
    }

    pub fn syncing_refreshes_detail(&self, generation: u64, skill_id: u64) -> Option<bool> {
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
pub struct ExpertHubState {
    pub section: ExpertHubSection,
    pub skill_tab: SkillTab,
    pub search_draft: String,
    pub submitted_search: String,
    pub market_tag_id: Option<u64>,
    pub market_company_tag_id: Option<u64>,
    pub system_filter: SkillListFilter,
    pub mine_filter: SkillListFilter,
    pub mine_tag_ids_draft: Vec<u64>,
    pub submitted_mine_tag_ids: Vec<u64>,
    pub mine_company_tag_ids_draft: Vec<u64>,
    pub submitted_mine_company_tag_ids: Vec<u64>,
    pub mine_statuses_draft: Vec<MineSkillStatus>,
    pub submitted_mine_statuses: Vec<MineSkillStatus>,
    pub mine_filter_menu: Option<MineFilterMenu>,
    pub platform_tag_catalog: LoadState<Vec<super::SkillTag>>,
    pub company_tag_catalog: LoadState<Vec<super::SkillTag>>,
    pub system_page: u32,
    pub mine_page: u32,
    pub marketplace: LoadState<super::CursorPage<super::MarketplaceSkill>>,
    pub installed: LoadState<Vec<super::InstalledSkill>>,
    pub system: LoadState<super::Page<super::SystemSkill>>,
    pub mine: LoadState<super::Page<super::TenantSkill>>,
    pub request_generation: u64,
    pub installed_request_generation: u64,
    pub tag_catalog_generation: u64,
    pub detail_generation: u64,
    pub action_generation: u64,
    pub cache_sync_generation: u64,
    pub function_upgrade: Option<u64>,
    pub main_upgrade: Option<u64>,
    pub function_cache_retry: Vec<u64>,
    pub account_id: Option<String>,
    pub detail: Option<SkillDetailState>,
    pub action: Option<SkillActionState>,
    pub version_confirmation: Option<SkillVersionAction>,
    pub cache_sync: SkillCacheSyncState,
    pub package_status: ShijuPackageStatus,
    pub install_guide: ShijuInstallGuideState,
    pub public_market: PublicSkillMarketState,
    pub connectors: super::connectors::ConnectorsState,
    pub knowledge: super::knowledge::KnowledgeState,
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
    pub fn select_market_tag(&mut self, tag_id: Option<u64>) {
        self.market_tag_id = tag_id;
    }

    pub fn select_market_company_tag(&mut self, tag_id: Option<u64>) {
        self.market_company_tag_id = tag_id;
    }

    pub fn toggle_mine_tag(&mut self, tag_id: u64, option_order: &[u64]) {
        toggle_ordered_ids(&mut self.mine_tag_ids_draft, tag_id, option_order);
    }

    pub fn toggle_mine_company_tag(&mut self, tag_id: u64, option_order: &[u64]) {
        toggle_ordered_ids(&mut self.mine_company_tag_ids_draft, tag_id, option_order);
    }

    pub fn toggle_mine_status(&mut self, status: MineSkillStatus) {
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

    pub fn toggle_mine_filter_menu(&mut self, menu: MineFilterMenu) {
        self.mine_filter_menu = (self.mine_filter_menu != Some(menu)).then_some(menu);
    }

    pub fn close_mine_filter_menu(&mut self) {
        self.mine_filter_menu = None;
    }

    pub fn submit_mine_filters(&mut self) {
        self.mine_filter.submit();
        self.submitted_mine_tag_ids.clone_from(&self.mine_tag_ids_draft);
        self.submitted_mine_company_tag_ids.clone_from(&self.mine_company_tag_ids_draft);
        self.submitted_mine_statuses.clone_from(&self.mine_statuses_draft);
        self.close_mine_filter_menu();
    }

    pub fn reset_mine_filters(&mut self) {
        self.mine_filter.reset();
        self.mine_tag_ids_draft.clear();
        self.submitted_mine_tag_ids.clear();
        self.mine_company_tag_ids_draft.clear();
        self.submitted_mine_company_tag_ids.clear();
        self.mine_statuses_draft = MineSkillStatus::DEFAULT.to_vec();
        self.submitted_mine_statuses = MineSkillStatus::DEFAULT.to_vec();
        self.close_mine_filter_menu();
    }

    pub fn reset_navigation(&mut self) {
        self.install_guide.close();
        self.section = ExpertHubSection::Skills;
        self.skill_tab = SkillTab::Marketplace;
    }

    /// 账号切换时清除账号范围数据并递增代次，使旧响应全部失效。
    pub fn clear_for_account(&mut self, account_id: Option<String>) {
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
        self.function_upgrade = None;
        self.main_upgrade = None;
        self.function_cache_retry.clear();
        self.version_confirmation = None;
        self.cache_sync = SkillCacheSyncState::Idle;
        self.package_status.cancel_refresh();
        self.install_guide.reset_requests();
        self.connectors.clear(self.account_id.clone());
        self.knowledge.clear(self.account_id.clone());
    }

    pub fn next_request_generation(&mut self) -> u64 {
        self.request_generation = self.request_generation.wrapping_add(1);
        self.request_generation
    }

    pub fn next_installed_request_generation(&mut self) -> u64 {
        self.installed_request_generation = self.installed_request_generation.wrapping_add(1);
        self.installed_request_generation
    }

    pub fn next_tag_catalog_generation(&mut self) -> u64 {
        self.tag_catalog_generation = self.tag_catalog_generation.wrapping_add(1);
        self.tag_catalog_generation
    }

    pub fn next_detail_generation(&mut self) -> u64 {
        self.detail_generation = self.detail_generation.wrapping_add(1);
        self.detail_generation
    }

    pub fn skill_operations_busy(&self) -> bool {
        self.action.is_some()
            || self.version_confirmation.is_some()
            || self.cache_sync.syncing_skill_id().is_some()
            || self.function_upgrade.is_some()
            || self.main_upgrade.is_some()
            || self.install_guide.is_busy()
    }

    /// 为技能启动互斥动作并返回新代次；已有动作时返回 `None`。
    pub fn start_action(&mut self, skill_id: u64, kind: SkillActionKind) -> Option<u64> {
        if self.skill_operations_busy() {
            return None;
        }
        self.action_generation = self.action_generation.wrapping_add(1);
        let generation = self.action_generation;
        self.action = Some(SkillActionState { skill_id, generation, kind });
        Some(generation)
    }

    pub fn start_cache_sync(&mut self, skill_id: u64, refresh_detail: bool) -> Option<u64> {
        if self.cache_sync.syncing_skill_id().is_some() {
            return None;
        }
        self.cache_sync_generation = self.cache_sync_generation.wrapping_add(1);
        let generation = self.cache_sync_generation;
        self.cache_sync = SkillCacheSyncState::Syncing { generation, skill_id, refresh_detail };
        Some(generation)
    }

    /// 仅完成与当前代次和技能匹配的缓存同步，迟到结果保持无效。
    pub fn finish_cache_sync(
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

    pub fn request_version_action(&mut self, action: SkillVersionAction) -> Option<u64> {
        if self.skill_operations_busy() {
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

    pub fn cancel_version_action(&mut self) {
        self.version_confirmation = None;
    }

    pub fn confirm_version_action(&mut self) -> Option<u64> {
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
            function_upgrade: None,
            main_upgrade: None,
            function_cache_retry: Vec::new(),
            account_id: None,
            detail: None,
            action: None,
            version_confirmation: None,
            cache_sync: SkillCacheSyncState::Idle,
            package_status: ShijuPackageStatus::default(),
            install_guide: ShijuInstallGuideState::default(),
            public_market: PublicSkillMarketState::default(),
            connectors: super::connectors::ConnectorsState::default(),
            knowledge: super::knowledge::KnowledgeState::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// 识矩包服务端版本刷新状态。
pub enum ShijuPackageRefreshState {
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
pub struct ShijuPackageStatus {
    pub local_version: Option<String>,
    pub server_version: Option<String>,
    pub checked_at_unix: Option<u64>,
    pub refresh: ShijuPackageRefreshState,
    refresh_generation: u64,
}

impl ShijuPackageStatus {
    pub fn from_sources(
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

    pub fn set_local_version(&mut self, local_version: Option<String>) {
        self.local_version = local_version;
    }

    pub fn start_refresh(&mut self) -> u64 {
        self.refresh_generation = self.refresh_generation.wrapping_add(1);
        let generation = self.refresh_generation;
        self.refresh = ShijuPackageRefreshState::Refreshing { generation };
        generation
    }

    pub fn is_refreshing(&self, generation: u64) -> bool {
        matches!(
            self.refresh,
            ShijuPackageRefreshState::Refreshing {
                generation: current,
            } if current == generation
        )
    }

    /// 仅接受当前刷新代次的结果，并据此更新服务端版本或失败状态。
    pub fn finish_refresh(
        &mut self,
        generation: u64,
        result: Result<super::PackageVersionCacheEntry, super::SkillHubError>,
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

    pub fn cancel_refresh(&mut self) {
        self.refresh_generation = self.refresh_generation.wrapping_add(1);
        self.refresh = ShijuPackageRefreshState::Idle;
    }
}
