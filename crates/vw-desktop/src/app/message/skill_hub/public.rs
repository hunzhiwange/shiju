//! 处理公共技能市场的检索、详情、安装与批量操作消息。

use iced::Task;
use vw_gateway_client::{
    DesktopSkillCatalogEntryDto, SkillMarketCategoryDto, SkillMarketCoordinateDto,
    SkillMarketInstallScopeDto, SkillMarketInstallSelectionDto, SkillMarketInstallTargetDto,
    SkillMarketSkillDetailDto, SkillMarketSkillDto, SkillMarketSkillFileDto,
    SkillMarketSkillLocatorDto, SkillMarketSkillPageDto, SkillMarketSkillQuery,
    SkillMarketSkillSetDetailDto, SkillMarketSkillSetPageDto, SkillMarketSort,
    SkillMarketSourceDto,
};

use crate::app::session_gateway::{
    gateway_skill_delete_async, gateway_skill_market_categories_async,
    gateway_skill_market_install_to_targets_async, gateway_skill_market_recommended_async,
    gateway_skill_market_skill_detail_async, gateway_skill_market_skill_file_async,
    gateway_skill_market_skill_set_detail_async, gateway_skill_market_skill_set_install_async,
    gateway_skill_market_skill_sets_async, gateway_skill_market_skills_async,
    gateway_skill_market_sources_async, gateway_skill_set_enabled_async, gateway_skills_async,
};
use crate::app::skill_hub::{
    LoadState, PublicBatchActionKind, PublicBatchActionState, PublicBatchSummary, PublicDetailMode,
    PublicDetailTarget, PublicInstallAvailability, PublicInstallState, PublicSkillMarketState,
    PublicSkillTab, SkillInstallScope, SkillInstallTarget, parse_public_markdown,
};
use crate::app::{App, Message};

use super::SkillHubMessage;

const MARKET_PAGE_SIZE: u32 = 24;

#[derive(Debug, Clone)]
#[allow(dead_code)]
/// 公共技能市场的用户输入、加载结果与安装操作消息。
pub(crate) enum PublicSkillMessage {
    LoadSources,
    SourcesLoaded {
        generation: u64,
        result: Result<Vec<SkillMarketSourceDto>, String>,
    },
    SelectSource(String),
    SelectTab(PublicSkillTab),
    RotateFeatured,
    QueryDraftChanged(String),
    SubmitQuery,
    SelectCategory(Option<String>),
    SelectSort(SkillMarketSort),
    PreviousPage,
    NextPage,
    Retry,
    OpenInstalled,
    OpenSkillDetail(SkillMarketSkillLocatorDto),
    OpenSuiteDetail {
        source: String,
        slug: String,
    },
    CloseDetail,
    SetDetailMode(PublicDetailMode),
    SelectDetailInstallScope(SkillInstallScope),
    SelectDetailInstallProject(String),
    ToggleDetailInstallTarget(SkillInstallTarget),
    InstallCurrent,
    SkillDetailLoaded {
        locator: SkillMarketSkillLocatorDto,
        generation: u64,
        result: Result<SkillMarketSkillDetailDto, String>,
    },
    SkillFileLoaded {
        locator: SkillMarketSkillLocatorDto,
        generation: u64,
        result: Result<SkillMarketSkillFileDto, String>,
    },
    SuiteDetailLoaded {
        source: String,
        slug: String,
        generation: u64,
        result: Result<SkillMarketSkillSetDetailDto, String>,
    },
    InstallFinished {
        target: PublicDetailTarget,
        generation: u64,
        project_path: String,
        result: Result<Vec<String>, String>,
    },
    InstalledRefreshed {
        generation: u64,
        result: Result<Vec<DesktopSkillCatalogEntryDto>, String>,
    },
    CloseInstalled,
    SetInstalledSelected {
        skill_id: String,
        selected: bool,
    },
    SelectAllInstalled,
    ClearInstalledSelection,
    StartBatchEnabled(bool),
    ToggleInstalled {
        skill_id: String,
        enabled: bool,
    },
    RequestBatchDelete,
    CancelBatchDelete,
    ConfirmBatchDelete,
    BatchItemFinished {
        generation: u64,
        project_path: String,
        skill_id: String,
        result: Result<(), String>,
    },
    RecommendedLoaded {
        source: String,
        generation: u64,
        result: Result<Vec<SkillMarketSkillDto>, String>,
    },
    CategoriesLoaded {
        source: String,
        generation: u64,
        result: Result<Vec<SkillMarketCategoryDto>, String>,
    },
    MarketLoaded {
        source: String,
        generation: u64,
        result: Result<SkillMarketSkillPageDto, String>,
    },
    SuitesLoaded {
        source: String,
        generation: u64,
        result: Result<SkillMarketSkillSetPageDto, String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 归约器产生的网关请求描述；实际异步执行与状态变更保持分离。
pub(crate) enum PublicRequest {
    Sources {
        generation: u64,
    },
    Recommended {
        source: String,
        generation: u64,
    },
    Categories {
        source: String,
        generation: u64,
    },
    Market {
        source: String,
        generation: u64,
        query: SkillMarketSkillQuery,
    },
    Suites {
        source: String,
        generation: u64,
        query: SkillMarketSkillQuery,
    },
    SkillDetail {
        locator: SkillMarketSkillLocatorDto,
        generation: u64,
    },
    SkillFile {
        locator: SkillMarketSkillLocatorDto,
        generation: u64,
    },
    SuiteDetail {
        source: String,
        slug: String,
        generation: u64,
    },
    InstallSkill {
        project_path: String,
        coordinate: SkillMarketCoordinateDto,
        installations: Vec<SkillMarketInstallSelectionDto>,
        target: PublicDetailTarget,
        generation: u64,
    },
    InstallSuite {
        project_path: String,
        source: String,
        slug: String,
        target: PublicDetailTarget,
        generation: u64,
    },
    RefreshInstalled {
        project_path: String,
        generation: u64,
    },
    SetSkillEnabled {
        project_path: String,
        skill_id: String,
        enabled: bool,
        generation: u64,
    },
    DeleteSkill {
        project_path: String,
        skill_id: String,
        generation: u64,
    },
}

/// 归约公共市场消息，并批量派发其产生的网关请求。
pub(crate) fn update(app: &mut App, message: PublicSkillMessage) -> Task<Message> {
    if let PublicSkillMessage::SelectDetailInstallProject(project_path) = &message {
        let project_path = project_path.trim();
        if project_path.is_empty() || !known_install_project(app, project_path) {
            return app.show_warning_toast("请选择当前或最近使用的项目");
        }
        let Some(detail) = app.expert_hub.public_market.detail.as_mut() else {
            return Task::none();
        };
        if detail.install_is_busy() {
            return app.show_warning_toast("安装进行中，暂时无法切换项目");
        }
        if !detail.select_install_project_path(project_path.to_string()) {
            return Task::none();
        }
        app.expert_hub.public_market.installed_generation =
            app.expert_hub.public_market.installed_generation.wrapping_add(1);
        let generation = app.expert_hub.public_market.installed_generation;
        app.expert_hub.public_market.installed = LoadState::Loading { generation };
        return dispatch(PublicRequest::RefreshInstalled {
            project_path: project_path.to_string(),
            generation,
        });
    }
    if matches!(message, PublicSkillMessage::InstallCurrent) {
        let project_path = app
            .expert_hub
            .public_market
            .detail
            .as_ref()
            .and_then(|detail| detail.selected_install_project_path.clone());
        return match begin_install(&mut app.expert_hub.public_market, project_path.as_deref()) {
            Ok(request) => dispatch(request),
            Err(error) => app.show_warning_toast(error),
        };
    }
    if let PublicSkillMessage::StartBatchEnabled(enabled) = &message {
        return match begin_batch_action(
            &mut app.expert_hub.public_market,
            app.project_path.as_deref(),
            if *enabled { PublicBatchActionKind::Enable } else { PublicBatchActionKind::Disable },
        ) {
            Ok(requests) => Task::batch(requests.into_iter().map(dispatch)),
            Err(error) => app.show_warning_toast(error),
        };
    }
    if let PublicSkillMessage::ToggleInstalled { skill_id, enabled } = &message {
        return match begin_action_for_ids(
            &mut app.expert_hub.public_market,
            app.project_path.as_deref(),
            if *enabled { PublicBatchActionKind::Enable } else { PublicBatchActionKind::Disable },
            vec![skill_id.clone()],
        ) {
            Ok(requests) => Task::batch(requests.into_iter().map(dispatch)),
            Err(error) => app.show_warning_toast(error),
        };
    }
    if matches!(message, PublicSkillMessage::ConfirmBatchDelete) {
        if !app.expert_hub.public_market.delete_confirmation_open {
            return Task::none();
        }
        app.expert_hub.public_market.delete_confirmation_open = false;
        return match begin_batch_action(
            &mut app.expert_hub.public_market,
            app.project_path.as_deref(),
            PublicBatchActionKind::Delete,
        ) {
            Ok(requests) => Task::batch(requests.into_iter().map(dispatch)),
            Err(error) => app.show_warning_toast(error),
        };
    }
    let install_feedback = match &message {
        PublicSkillMessage::InstallFinished { target, generation, result, .. }
            if install_response_is_current(&app.expert_hub.public_market, target, *generation) =>
        {
            Some(result.as_ref().map(|_| ()).map_err(Clone::clone))
        }
        _ => None,
    };
    let refresh_error = match &message {
        PublicSkillMessage::InstalledRefreshed { generation, result }
            if *generation == app.expert_hub.public_market.installed_generation =>
        {
            result.as_ref().err().cloned()
        }
        _ => None,
    };
    let batch_feedback = match &message {
        PublicSkillMessage::BatchItemFinished { generation, result, .. } => app
            .expert_hub
            .public_market
            .batch_action
            .as_ref()
            .filter(|action| action.generation == *generation && action.pending.len() == 1)
            .map(|action| {
                let succeeded = action.succeeded + usize::from(result.is_ok());
                let failed = action.failures.len() + usize::from(result.is_err());
                PublicBatchSummary { succeeded, failed }
            }),
        _ => None,
    };
    let open_installed = matches!(message, PublicSkillMessage::OpenInstalled);
    let opens_detail = matches!(
        &message,
        PublicSkillMessage::OpenSkillDetail(_) | PublicSkillMessage::OpenSuiteDetail { .. }
    );
    let default_project_path = default_install_project_path(app);
    let mut requests = reduce(&mut app.expert_hub.public_market, message);
    let detail_opened = opens_detail
        && requests.iter().any(|request| {
            matches!(request, PublicRequest::SkillDetail { .. } | PublicRequest::SuiteDetail { .. })
        });
    if detail_opened {
        if let Some(detail) = app.expert_hub.public_market.detail.as_mut() {
            detail.selected_install_project_path = default_project_path.clone();
        }
    }
    if detail_opened
        && matches!(app.expert_hub.public_market.installed, LoadState::Idle | LoadState::Error(_))
        && let Some(project_path) = default_project_path
    {
        app.expert_hub.public_market.installed_generation =
            app.expert_hub.public_market.installed_generation.wrapping_add(1);
        let generation = app.expert_hub.public_market.installed_generation;
        app.expert_hub.public_market.installed = LoadState::Loading { generation };
        requests.push(PublicRequest::RefreshInstalled { project_path, generation });
    }
    if open_installed {
        app.expert_hub.public_market.installed_generation =
            app.expert_hub.public_market.installed_generation.wrapping_add(1);
        let generation = app.expert_hub.public_market.installed_generation;
        app.expert_hub.public_market.installed = LoadState::Loading { generation };
        requests.push(PublicRequest::RefreshInstalled {
            project_path: app.project_path.clone().unwrap_or_default(),
            generation,
        });
    }
    let mut tasks = requests.into_iter().map(dispatch).collect::<Vec<_>>();
    if let Some(result) = install_feedback {
        tasks.push(match result {
            Ok(()) => app.show_success_toast("技能安装成功"),
            Err(error) => app.show_error_toast(format!("技能安装失败：{error}")),
        });
    }
    if let Some(error) = refresh_error {
        tasks.push(app.show_warning_toast(format!("技能已安装，但刷新本地目录失败：{error}")));
    }
    if let Some(summary) = batch_feedback {
        let message = if summary.failed == 0 {
            format!("批量操作完成：成功 {} 项", summary.succeeded)
        } else {
            format!("批量操作完成：成功 {} 项，失败 {} 项", summary.succeeded, summary.failed)
        };
        tasks.push(if summary.failed == 0 {
            app.show_success_toast(message)
        } else {
            app.show_warning_toast(message)
        });
    }
    Task::batch(tasks)
}

/// 在公共市场尚未初始化时加载来源与当前页，已加载时保持状态不变。
pub(crate) fn ensure_loaded(app: &mut App) -> Task<Message> {
    if matches!(app.expert_hub.public_market.sources, LoadState::Idle | LoadState::Error(_)) {
        update(app, PublicSkillMessage::LoadSources)
    } else {
        Task::none()
    }
}

/// 纯化处理公共市场状态转换，并返回后续请求而不直接执行网络操作。
pub(crate) fn reduce(
    state: &mut PublicSkillMarketState,
    message: PublicSkillMessage,
) -> Vec<PublicRequest> {
    match message {
        PublicSkillMessage::LoadSources => vec![load_sources(state)],
        PublicSkillMessage::SourcesLoaded { generation, result } => {
            if state.sources_generation != generation {
                return Vec::new();
            }
            match result {
                Ok(sources) => {
                    let default_source = sources
                        .iter()
                        .find(|source| source.source == "skillhub")
                        .or_else(|| sources.first())
                        .map(|source| source.source.clone());
                    state.sources = LoadState::Loaded(sources);
                    if let Some(source) = default_source {
                        state.current_source = None;
                        state.select_source(&source);
                        load_current_tab(state, false)
                    } else {
                        state.current_source = None;
                        Vec::new()
                    }
                }
                Err(error) => {
                    state.sources = LoadState::Error(error);
                    Vec::new()
                }
            }
        }
        PublicSkillMessage::SelectSource(source) => {
            if state.select_source(&source) {
                load_current_tab(state, false)
            } else {
                Vec::new()
            }
        }
        PublicSkillMessage::SelectTab(tab) => {
            state.tab = tab;
            load_current_tab(state, false)
        }
        PublicSkillMessage::RotateFeatured => {
            state.rotate_featured();
            Vec::new()
        }
        PublicSkillMessage::QueryDraftChanged(query) => {
            state.query.keyword_draft = query;
            Vec::new()
        }
        PublicSkillMessage::SubmitQuery => {
            state.submit_query();
            if state.tab == PublicSkillTab::Recommended {
                state.tab = PublicSkillTab::SkillHub;
            }
            load_current_tab(state, true)
        }
        PublicSkillMessage::SelectCategory(category) => {
            state.query.category = category.filter(|category| !category.trim().is_empty());
            state.submit_query();
            load_current_tab(state, true)
        }
        PublicSkillMessage::SelectSort(sort) => {
            state.query.sort = sort;
            state.submit_query();
            load_current_tab(state, true)
        }
        PublicSkillMessage::PreviousPage => change_page(state, false),
        PublicSkillMessage::NextPage => change_page(state, true),
        PublicSkillMessage::Retry => {
            if matches!(state.sources, LoadState::Error(_) | LoadState::Idle) {
                vec![load_sources(state)]
            } else {
                load_current_tab(state, true)
            }
        }
        PublicSkillMessage::OpenInstalled => {
            state.installed_open = true;
            state.close_detail();
            Vec::new()
        }
        PublicSkillMessage::CloseInstalled => {
            state.installed_open = false;
            state.delete_confirmation_open = false;
            Vec::new()
        }
        PublicSkillMessage::SetInstalledSelected { skill_id, selected } => {
            state.set_installed_selected(&skill_id, selected);
            Vec::new()
        }
        PublicSkillMessage::SelectAllInstalled => {
            state.select_all_installed();
            Vec::new()
        }
        PublicSkillMessage::ClearInstalledSelection => {
            state.clear_installed_selection();
            Vec::new()
        }
        PublicSkillMessage::RequestBatchDelete => {
            if !state.installed_selection.is_empty() && state.batch_action.is_none() {
                state.delete_confirmation_open = true;
            }
            Vec::new()
        }
        PublicSkillMessage::CancelBatchDelete => {
            state.delete_confirmation_open = false;
            Vec::new()
        }
        PublicSkillMessage::InstallCurrent
        | PublicSkillMessage::StartBatchEnabled(_)
        | PublicSkillMessage::ToggleInstalled { .. }
        | PublicSkillMessage::ConfirmBatchDelete => Vec::new(),
        PublicSkillMessage::OpenSkillDetail(locator) => {
            if state.current_source.as_deref() != Some(&locator.source) {
                return Vec::new();
            }
            let generation = state.open_skill_detail(locator.clone());
            vec![
                PublicRequest::SkillDetail { locator: locator.clone(), generation },
                PublicRequest::SkillFile { locator, generation },
            ]
        }
        PublicSkillMessage::OpenSuiteDetail { source, slug } => {
            if state.current_source.as_deref() != Some(&source) {
                return Vec::new();
            }
            let generation = state.open_suite_detail(source.clone(), slug.clone());
            vec![PublicRequest::SuiteDetail { source, slug, generation }]
        }
        PublicSkillMessage::CloseDetail => {
            state.close_detail();
            Vec::new()
        }
        PublicSkillMessage::SetDetailMode(mode) => {
            if let Some(detail) = state.detail.as_mut() {
                detail.mode = mode;
            }
            Vec::new()
        }
        PublicSkillMessage::SelectDetailInstallScope(scope) => {
            if let Some(detail) = state.detail.as_mut() {
                detail.select_install_scope(scope);
            }
            Vec::new()
        }
        PublicSkillMessage::SelectDetailInstallProject(project_path) => {
            if let Some(detail) = state.detail.as_mut() {
                detail.select_install_project_path(project_path);
            }
            Vec::new()
        }
        PublicSkillMessage::ToggleDetailInstallTarget(target) => {
            if let Some(detail) = state.detail.as_mut() {
                detail.toggle_install_target(target);
            }
            Vec::new()
        }
        PublicSkillMessage::SkillDetailLoaded { locator, generation, result } => {
            if let Some(detail) = current_skill_detail(state, &locator, generation) {
                detail.skill_detail =
                    result.map(LoadState::Loaded).unwrap_or_else(LoadState::Error);
            }
            Vec::new()
        }
        PublicSkillMessage::SkillFileLoaded { locator, generation, result } => {
            if let Some(detail) = current_skill_detail(state, &locator, generation) {
                match result {
                    Ok(file) => {
                        detail.markdown_items = parse_public_markdown(&file.content);
                        detail.skill_file = LoadState::Loaded(file);
                    }
                    Err(error) => detail.skill_file = LoadState::Error(error),
                }
            }
            Vec::new()
        }
        PublicSkillMessage::SuiteDetailLoaded { source, slug, generation, result } => {
            if let Some(detail) = current_suite_detail(state, &source, &slug, generation) {
                match result {
                    Ok(suite) => {
                        detail.markdown_items = parse_public_markdown(&suite.content);
                        detail.suite_detail = LoadState::Loaded(suite);
                    }
                    Err(error) => detail.suite_detail = LoadState::Error(error),
                }
            }
            Vec::new()
        }
        PublicSkillMessage::InstallFinished { target, generation, project_path, result } => {
            if !install_response_is_current(state, &target, generation) {
                return Vec::new();
            }
            let detail = state.detail.as_mut().expect("current detail must exist");
            match result {
                Ok(paths) => {
                    detail.install = PublicInstallState::Installed { paths };
                    if project_path.trim().is_empty() {
                        return Vec::new();
                    }
                    state.installed_generation = state.installed_generation.wrapping_add(1);
                    let generation = state.installed_generation;
                    state.installed = LoadState::Loading { generation };
                    vec![PublicRequest::RefreshInstalled { project_path, generation }]
                }
                Err(_) => {
                    detail.install = PublicInstallState::Idle;
                    Vec::new()
                }
            }
        }
        PublicSkillMessage::InstalledRefreshed { generation, result } => {
            if state.installed_generation == generation {
                match result {
                    Ok(items) => {
                        let manageable = items
                            .iter()
                            .filter(|item| item.installed && item.source_path.is_some())
                            .map(|item| item.id.as_str())
                            .collect::<std::collections::BTreeSet<_>>();
                        state
                            .installed_selection
                            .retain(|skill_id| manageable.contains(skill_id.as_str()));
                        state.installed = LoadState::Loaded(items);
                    }
                    Err(error) => state.installed = LoadState::Error(error),
                }
            }
            Vec::new()
        }
        PublicSkillMessage::BatchItemFinished { generation, project_path, skill_id, result } => {
            finish_batch_item(state, generation, project_path, skill_id, result)
        }
        PublicSkillMessage::RecommendedLoaded { source, generation, result } => {
            if response_is_current(state, &source, generation, state.recommended_generation) {
                state.featured_offset = 0;
                state.recommended = result.map(LoadState::Loaded).unwrap_or_else(LoadState::Error);
            }
            Vec::new()
        }
        PublicSkillMessage::CategoriesLoaded { source, generation, result } => {
            if response_is_current(state, &source, generation, state.categories_generation) {
                state.categories = result.map(LoadState::Loaded).unwrap_or_else(LoadState::Error);
            }
            Vec::new()
        }
        PublicSkillMessage::MarketLoaded { source, generation, result } => {
            if response_is_current(state, &source, generation, state.market_generation) {
                state.market = result.map(LoadState::Loaded).unwrap_or_else(LoadState::Error);
            }
            Vec::new()
        }
        PublicSkillMessage::SuitesLoaded { source, generation, result } => {
            if response_is_current(state, &source, generation, state.suites_generation) {
                state.suites = result.map(LoadState::Loaded).unwrap_or_else(LoadState::Error);
            }
            Vec::new()
        }
    }
}

fn load_sources(state: &mut PublicSkillMarketState) -> PublicRequest {
    state.sources_generation = state.sources_generation.wrapping_add(1);
    let generation = state.sources_generation;
    state.sources = LoadState::Loading { generation };
    PublicRequest::Sources { generation }
}

fn load_current_tab(state: &mut PublicSkillMarketState, force: bool) -> Vec<PublicRequest> {
    let Some(source) = state.current_source.clone() else {
        return Vec::new();
    };
    match state.tab {
        PublicSkillTab::Recommended => {
            if !force && matches!(state.recommended, LoadState::Loaded(_)) {
                Vec::new()
            } else {
                state.recommended_generation = state.recommended_generation.wrapping_add(1);
                let generation = state.recommended_generation;
                state.recommended = LoadState::Loading { generation };
                vec![PublicRequest::Recommended { source, generation }]
            }
        }
        PublicSkillTab::SkillHub => {
            let mut requests = Vec::new();
            if force || !matches!(state.categories, LoadState::Loaded(_)) {
                state.categories_generation = state.categories_generation.wrapping_add(1);
                let generation = state.categories_generation;
                state.categories = LoadState::Loading { generation };
                requests.push(PublicRequest::Categories { source: source.clone(), generation });
            }
            if force || !matches!(state.market, LoadState::Loaded(_)) {
                state.market_generation = state.market_generation.wrapping_add(1);
                let generation = state.market_generation;
                state.market = LoadState::Loading { generation };
                requests.push(PublicRequest::Market {
                    source,
                    generation,
                    query: market_query(state, state.query.page),
                });
            }
            requests
        }
        PublicSkillTab::Suites => {
            if !force && matches!(state.suites, LoadState::Loaded(_)) {
                Vec::new()
            } else {
                state.suites_generation = state.suites_generation.wrapping_add(1);
                let generation = state.suites_generation;
                state.suites = LoadState::Loading { generation };
                vec![PublicRequest::Suites {
                    source,
                    generation,
                    query: market_query(state, state.suites_page),
                }]
            }
        }
    }
}

fn change_page(state: &mut PublicSkillMarketState, next: bool) -> Vec<PublicRequest> {
    let changed = match state.tab {
        PublicSkillTab::SkillHub => {
            let can_advance = matches!(
                &state.market,
                LoadState::Loaded(page)
                    if u64::from(page.page) * u64::from(page.page_size) < page.total
            );
            if next && can_advance {
                state.query.page = state.query.page.saturating_add(1);
                true
            } else if !next && state.query.page > 1 {
                state.query.page -= 1;
                true
            } else {
                false
            }
        }
        PublicSkillTab::Suites => {
            let can_advance = matches!(
                &state.suites,
                LoadState::Loaded(page)
                    if u64::from(page.page) * u64::from(page.page_size) < page.total
            );
            if next && can_advance {
                state.suites_page = state.suites_page.saturating_add(1);
                true
            } else if !next && state.suites_page > 1 {
                state.suites_page -= 1;
                true
            } else {
                false
            }
        }
        PublicSkillTab::Recommended => false,
    };
    if changed { load_current_tab(state, true) } else { Vec::new() }
}

fn market_query(state: &PublicSkillMarketState, page: u32) -> SkillMarketSkillQuery {
    SkillMarketSkillQuery {
        keyword: (!state.query.submitted_keyword.is_empty())
            .then(|| state.query.submitted_keyword.clone()),
        category: state.query.category.clone(),
        namespace: None,
        sort: state.query.sort,
        page,
        page_size: MARKET_PAGE_SIZE,
    }
}

fn response_is_current(
    state: &PublicSkillMarketState,
    source: &str,
    generation: u64,
    current_generation: u64,
) -> bool {
    state.current_source.as_deref() == Some(source) && generation == current_generation
}

fn current_skill_detail<'a>(
    state: &'a mut PublicSkillMarketState,
    locator: &SkillMarketSkillLocatorDto,
    generation: u64,
) -> Option<&'a mut crate::app::skill_hub::PublicDetailState> {
    let detail = state.detail.as_mut()?;
    (detail.generation == generation && detail.target == PublicDetailTarget::Skill(locator.clone()))
        .then_some(detail)
}

fn current_suite_detail<'a>(
    state: &'a mut PublicSkillMarketState,
    source: &str,
    slug: &str,
    generation: u64,
) -> Option<&'a mut crate::app::skill_hub::PublicDetailState> {
    let detail = state.detail.as_mut()?;
    (detail.generation == generation
        && detail.target
            == PublicDetailTarget::Suite { source: source.to_string(), slug: slug.to_string() })
    .then_some(detail)
}

fn install_response_is_current(
    state: &PublicSkillMarketState,
    target: &PublicDetailTarget,
    generation: u64,
) -> bool {
    state.detail.as_ref().is_some_and(|detail| {
        detail.target == *target
            && matches!(
                detail.install,
                PublicInstallState::Installing {
                    generation: active,
                } if active == generation
            )
    })
}

fn default_install_project_path(app: &App) -> Option<String> {
    app.project_path
        .iter()
        .chain(app.recent_projects.iter())
        .map(|path| path.trim())
        .find(|path| !path.is_empty())
        .map(str::to_string)
}

fn known_install_project(app: &App, project_path: &str) -> bool {
    app.project_path
        .iter()
        .chain(app.recent_projects.iter())
        .any(|known| known.trim() == project_path)
}

/// 校验详情目标、项目范围和安装目标后开始一次安装请求。
pub(crate) fn begin_install(
    state: &mut PublicSkillMarketState,
    project_path: Option<&str>,
) -> Result<PublicRequest, String> {
    let detail = state.detail.as_ref().ok_or_else(|| "请先打开技能或套件详情".to_string())?;
    let project_missing = PublicSkillMarketState::install_availability(project_path)
        == PublicInstallAvailability::NoProject;
    match &detail.target {
        PublicDetailTarget::Skill(_) if detail.install_scope == SkillInstallScope::Project => {
            if project_missing {
                return Err("项目级安装需要先选择项目".to_string());
            }
        }
        PublicDetailTarget::Suite { .. } if project_missing => {
            return Err("请先选择项目再安装技能".to_string());
        }
        _ => {}
    }
    if matches!(detail.install, PublicInstallState::Installing { .. }) {
        return Err("安装正在进行中".to_string());
    }
    if matches!(detail.install, PublicInstallState::Installed { .. }) {
        return Err("所选目标已完成安装，请调整选择后重试".to_string());
    }
    let target = detail.target.clone();
    let install_target = match &target {
        PublicDetailTarget::Skill(locator) => {
            let LoadState::Loaded(skill_detail) = &detail.skill_detail else {
                return Err("技能详情尚未加载完成".to_string());
            };
            if skill_detail.skill.coordinate.locator != *locator {
                return Err("技能详情坐标与当前请求不一致".to_string());
            }
            let installations = detail
                .selected_install_targets
                .iter()
                .copied()
                .map(|target| install_selection(target, detail.install_scope))
                .collect::<Vec<_>>();
            if installations.is_empty() {
                return Err("请至少选择一个安装目标".to_string());
            }
            Some((skill_detail.skill.coordinate.clone(), installations))
        }
        PublicDetailTarget::Suite { .. } => {
            if !matches!(detail.suite_detail, LoadState::Loaded(_)) {
                return Err("套件详情尚未加载完成".to_string());
            }
            None
        }
    };
    let project_path =
        project_path.map(str::trim).filter(|path| !path.is_empty()).unwrap_or_default().to_string();

    state.install_generation = state.install_generation.wrapping_add(1);
    let generation = state.install_generation;
    state.detail.as_mut().expect("detail was validated").install =
        PublicInstallState::Installing { generation };
    match (target.clone(), install_target) {
        (PublicDetailTarget::Skill(_), Some((coordinate, installations))) => {
            Ok(PublicRequest::InstallSkill {
                project_path,
                coordinate,
                installations,
                target,
                generation,
            })
        }
        (PublicDetailTarget::Suite { source, slug }, None) => {
            Ok(PublicRequest::InstallSuite { project_path, source, slug, target, generation })
        }
        _ => Err("详情安装状态无效".to_string()),
    }
}

fn install_selection(
    target: SkillInstallTarget,
    scope: SkillInstallScope,
) -> SkillMarketInstallSelectionDto {
    let target = match target {
        SkillInstallTarget::VibeWindow => SkillMarketInstallTargetDto::VibeWindow,
        SkillInstallTarget::AgentSkills => SkillMarketInstallTargetDto::AgentSkills,
        SkillInstallTarget::Codex => SkillMarketInstallTargetDto::Codex,
        SkillInstallTarget::WorkBuddy => SkillMarketInstallTargetDto::WorkBuddy,
        SkillInstallTarget::ClaudeCode => SkillMarketInstallTargetDto::ClaudeCode,
        SkillInstallTarget::OpenCode => SkillMarketInstallTargetDto::OpenCode,
        SkillInstallTarget::GeminiCli => SkillMarketInstallTargetDto::GeminiCli,
        SkillInstallTarget::GitHubCopilot => SkillMarketInstallTargetDto::GitHubCopilot,
        SkillInstallTarget::Cursor => SkillMarketInstallTargetDto::Cursor,
        SkillInstallTarget::Windsurf => SkillMarketInstallTargetDto::Windsurf,
    };
    let scope = match scope {
        SkillInstallScope::Global => SkillMarketInstallScopeDto::Global,
        SkillInstallScope::Project => SkillMarketInstallScopeDto::Project,
    };
    SkillMarketInstallSelectionDto { target, scope }
}

/// 为当前选中的已安装技能启动批量启停或删除操作。
pub(crate) fn begin_batch_action(
    state: &mut PublicSkillMarketState,
    project_path: Option<&str>,
    kind: PublicBatchActionKind,
) -> Result<Vec<PublicRequest>, String> {
    let snapshot =
        state.start_batch_action(kind).ok_or_else(|| "请先选择可管理的已安装技能".to_string())?;
    Ok(batch_requests(
        project_path.unwrap_or_default(),
        snapshot.generation,
        snapshot.kind,
        snapshot.skill_ids,
    ))
}

fn begin_action_for_ids(
    state: &mut PublicSkillMarketState,
    project_path: Option<&str>,
    kind: PublicBatchActionKind,
    skill_ids: Vec<String>,
) -> Result<Vec<PublicRequest>, String> {
    if state.batch_action.is_some() {
        return Err("已有批量操作正在进行".to_string());
    }
    let manageable = match &state.installed {
        LoadState::Loaded(items) => skill_ids.iter().all(|skill_id| {
            items
                .iter()
                .any(|item| item.id == *skill_id && item.installed && item.source_path.is_some())
        }),
        _ => false,
    };
    if skill_ids.is_empty() || !manageable {
        return Err("只能操作 gateway 判定可管理的本地技能".to_string());
    }
    state.batch_generation = state.batch_generation.wrapping_add(1);
    let generation = state.batch_generation;
    state.batch_action = Some(PublicBatchActionState {
        generation,
        kind,
        pending: skill_ids.iter().cloned().collect(),
        succeeded: 0,
        failures: std::collections::BTreeMap::new(),
        preserve_selection: true,
    });
    state.last_batch_summary = None;
    Ok(batch_requests(project_path.unwrap_or_default(), generation, kind, skill_ids))
}

fn batch_requests(
    project_path: &str,
    generation: u64,
    kind: PublicBatchActionKind,
    skill_ids: Vec<String>,
) -> Vec<PublicRequest> {
    skill_ids
        .into_iter()
        .map(|skill_id| match kind {
            PublicBatchActionKind::Enable | PublicBatchActionKind::Disable => {
                PublicRequest::SetSkillEnabled {
                    project_path: project_path.to_string(),
                    skill_id,
                    enabled: kind == PublicBatchActionKind::Enable,
                    generation,
                }
            }
            PublicBatchActionKind::Delete => PublicRequest::DeleteSkill {
                project_path: project_path.to_string(),
                skill_id,
                generation,
            },
        })
        .collect()
}

fn finish_batch_item(
    state: &mut PublicSkillMarketState,
    generation: u64,
    project_path: String,
    skill_id: String,
    result: Result<(), String>,
) -> Vec<PublicRequest> {
    let Some(action) = state.batch_action.as_mut() else {
        return Vec::new();
    };
    if action.generation != generation || !action.pending.remove(&skill_id) {
        return Vec::new();
    }
    match result {
        Ok(()) => action.succeeded = action.succeeded.saturating_add(1),
        Err(error) => {
            action.failures.insert(skill_id, error);
        }
    }
    if !action.pending.is_empty() {
        return Vec::new();
    }

    let summary = PublicBatchSummary { succeeded: action.succeeded, failed: action.failures.len() };
    let preserve_selection = action.preserve_selection;
    let failures = action.failures.keys().cloned().collect();
    state.last_batch_summary = Some(summary);
    state.batch_action = None;
    if !preserve_selection {
        state.installed_selection = failures;
    }
    state.installed_generation = state.installed_generation.wrapping_add(1);
    let generation = state.installed_generation;
    state.installed = LoadState::Loading { generation };
    vec![PublicRequest::RefreshInstalled { project_path, generation }]
}

fn dispatch(request: PublicRequest) -> Task<Message> {
    match request {
        PublicRequest::Sources { generation } => {
            Task::perform(gateway_skill_market_sources_async(), move |result| {
                Message::SkillHub(SkillHubMessage::Public(PublicSkillMessage::SourcesLoaded {
                    generation,
                    result,
                }))
            })
        }
        PublicRequest::Recommended { source, generation } => {
            let requested_source = source.clone();
            Task::perform(
                async move { gateway_skill_market_recommended_async(&requested_source).await },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::RecommendedLoaded { source, generation, result },
                    ))
                },
            )
        }
        PublicRequest::Categories { source, generation } => {
            let requested_source = source.clone();
            Task::perform(
                async move { gateway_skill_market_categories_async(&requested_source).await },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::CategoriesLoaded { source, generation, result },
                    ))
                },
            )
        }
        PublicRequest::Market { source, generation, query } => {
            let requested_source = source.clone();
            Task::perform(
                async move { gateway_skill_market_skills_async(&requested_source, &query).await },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(PublicSkillMessage::MarketLoaded {
                        source,
                        generation,
                        result,
                    }))
                },
            )
        }
        PublicRequest::Suites { source, generation, query } => {
            let requested_source = source.clone();
            Task::perform(
                async move { gateway_skill_market_skill_sets_async(&requested_source, &query).await },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(PublicSkillMessage::SuitesLoaded {
                        source,
                        generation,
                        result,
                    }))
                },
            )
        }
        PublicRequest::SkillDetail { locator, generation } => {
            let requested_locator = locator.clone();
            Task::perform(
                async move { gateway_skill_market_skill_detail_async(&requested_locator).await },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::SkillDetailLoaded { locator, generation, result },
                    ))
                },
            )
        }
        PublicRequest::SkillFile { locator, generation } => {
            let requested_locator = locator.clone();
            Task::perform(
                async move { gateway_skill_market_skill_file_async(&requested_locator).await },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::SkillFileLoaded { locator, generation, result },
                    ))
                },
            )
        }
        PublicRequest::SuiteDetail { source, slug, generation } => {
            let requested_source = source.clone();
            let requested_slug = slug.clone();
            Task::perform(
                async move {
                    gateway_skill_market_skill_set_detail_async(&requested_source, &requested_slug)
                        .await
                },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::SuiteDetailLoaded { source, slug, generation, result },
                    ))
                },
            )
        }
        PublicRequest::InstallSkill {
            project_path,
            coordinate,
            installations,
            target,
            generation,
        } => {
            let requested_project = project_path.clone();
            Task::perform(
                async move {
                    gateway_skill_market_install_to_targets_async(
                        (!requested_project.is_empty()).then_some(requested_project.as_str()),
                        &coordinate,
                        &installations,
                    )
                    .await
                },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::InstallFinished {
                            target,
                            generation,
                            project_path,
                            result,
                        },
                    ))
                },
            )
        }
        PublicRequest::InstallSuite { project_path, source, slug, target, generation } => {
            let requested_project = project_path.clone();
            let requested_source = source.clone();
            let requested_slug = slug.clone();
            Task::perform(
                async move {
                    gateway_skill_market_skill_set_install_async(
                        &requested_project,
                        &requested_source,
                        &requested_slug,
                    )
                    .await
                },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::InstallFinished {
                            target,
                            generation,
                            project_path,
                            result,
                        },
                    ))
                },
            )
        }
        PublicRequest::RefreshInstalled { project_path, generation } => Task::perform(
            async move { gateway_skills_async(Some(&project_path)).await },
            move |result| {
                Message::SkillHub(SkillHubMessage::Public(PublicSkillMessage::InstalledRefreshed {
                    generation,
                    result,
                }))
            },
        ),
        PublicRequest::SetSkillEnabled { project_path, skill_id, enabled, generation } => {
            let requested_project = project_path.clone();
            let requested_skill = skill_id.clone();
            Task::perform(
                async move {
                    gateway_skill_set_enabled_async(
                        (!requested_project.is_empty()).then_some(requested_project.as_str()),
                        &requested_skill,
                        enabled,
                    )
                    .await
                    .map(|_| ())
                },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::BatchItemFinished {
                            generation,
                            project_path,
                            skill_id,
                            result,
                        },
                    ))
                },
            )
        }
        PublicRequest::DeleteSkill { project_path, skill_id, generation } => {
            let requested_project = project_path.clone();
            let requested_skill = skill_id.clone();
            Task::perform(
                async move {
                    gateway_skill_delete_async(
                        (!requested_project.is_empty()).then_some(requested_project.as_str()),
                        &requested_skill,
                    )
                    .await
                    .map(|_| ())
                },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::BatchItemFinished {
                            generation,
                            project_path,
                            skill_id,
                            result,
                        },
                    ))
                },
            )
        }
    }
}
